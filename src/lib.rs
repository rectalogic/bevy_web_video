use crossbeam_channel::unbounded;
use std::{cell::RefCell, collections::HashMap};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use wasm_bindgen::prelude::*;
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = unbounded();
        app.insert_resource(WebVideoRegistry { rx, tx })
            .add_systems(Update, (remove_unused_video_elements, handle_resize));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, render_videos.in_set(RenderSet::PrepareResources));
    }
}

// wasm on web is single threaded, so this should be OK
thread_local! {
    static VIDEO_ELEMENTS: RefCell<HashMap<AssetId<Image>, VideoElement>> =  RefCell::new(HashMap::default());
}

#[derive(Clone)]
struct VideoElement {
    element: web_sys::HtmlVideoElement,
    loaded: bool,
}

#[derive(Resource)]
pub struct WebVideoRegistry {
    rx: crossbeam_channel::Receiver<VideoSizeMessage>,
    tx: crossbeam_channel::Sender<VideoSizeMessage>,
}

#[derive(Clone, Component)]
pub struct WebVideo {
    asset_id: AssetId<Image>,
}

#[derive(Debug)]
pub struct WebVideoError {
    message: String,
}

#[derive(Copy, Clone)]
struct VideoSizeMessage(AssetId<Image>);

#[derive(Copy, Clone, Event)]
pub struct ResizeVideoEvent {
    pub asset_id: AssetId<Image>,
    pub width: u32,
    pub height: u32,
}

impl WebVideoRegistry {
    pub fn new_video_texture(
        &self,
        images: Res<Assets<Image>>,
    ) -> Result<(Handle<Image>, web_sys::HtmlVideoElement)> {
        let html_video_element = web_sys::window()
            .expect("window")
            .document()
            .expect("document")
            .create_element("video")
            .map_err(WebVideoError::from)?
            .dyn_into::<web_sys::HtmlVideoElement>()
            .expect("web_sys::HtmlVideoElement");

        let image_handle = images
            .get_handle_provider()
            .reserve_handle()
            .typed::<Image>();
        let asset_id = image_handle.id();

        let tx = self.tx.clone();
        let resize_cb = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::Event| {
            if let Err(err) = tx.send(VideoSizeMessage(asset_id)) {
                warn!("Failed to handle video resize: {err:?}");
            }
        });
        html_video_element
            .add_event_listener_with_callback("loadedmetadata", resize_cb.as_ref().unchecked_ref())
            .map_err(WebVideoError::from)?;
        html_video_element
            .add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref())
            .map_err(WebVideoError::from)?;
        resize_cb.forget();

        let playing_cb = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::Event| {
            VIDEO_ELEMENTS.with_borrow_mut(|ve| {
                if let Some(video_element) = ve.get_mut(&asset_id) {
                    video_element.loaded = true;
                }
            });
        });
        html_video_element
            .add_event_listener_with_callback("playing", playing_cb.as_ref().unchecked_ref())
            .map_err(WebVideoError::from)?;
        playing_cb.forget();

        VIDEO_ELEMENTS.with_borrow_mut(|ve| {
            ve.insert(
                asset_id,
                VideoElement {
                    element: html_video_element.clone(),
                    loaded: false,
                },
            )
        });
        Ok((image_handle, html_video_element))
    }

    pub fn get_video_element(asset_id: AssetId<Image>) -> Option<web_sys::HtmlVideoElement> {
        VIDEO_ELEMENTS.with_borrow(|ve| ve.get(&asset_id).map(|e| e.element.clone()))
    }
}

impl WebVideo {
    pub fn new(asset_id: AssetId<Image>) -> Result<Self> {
        if !VIDEO_ELEMENTS.with_borrow(|ve| ve.contains_key(&asset_id)) {
            Err(format!("Invalid AssetId {asset_id:?}").into())
        } else {
            Ok(Self { asset_id })
        }
    }

    pub fn asset_id(&self) -> AssetId<Image> {
        self.asset_id
    }

    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        WebVideoRegistry::get_video_element(self.asset_id)
    }
}

fn new_image(size: Extent3d) -> Image {
    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image
}

impl std::error::Error for WebVideoError {}

impl std::fmt::Display for WebVideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<JsValue> for WebVideoError {
    fn from(value: JsValue) -> Self {
        Self {
            message: format!("{value:?}"),
        }
    }
}

fn remove_unused_video_elements(mut events: EventReader<AssetEvent<Image>>) {
    for event in events.read() {
        if let AssetEvent::Unused { id: asset_id } = event {
            VIDEO_ELEMENTS.with_borrow_mut(|ve| ve.remove(asset_id));
        }
    }
}

fn handle_resize(
    mut commands: Commands,
    video_registry: Res<WebVideoRegistry>,
    videos: Query<(Entity, &WebVideo)>,
    mut images: ResMut<Assets<Image>>,
) {
    while let Ok(resize_message) = video_registry.rx.try_recv() {
        if let Some(size_event) = VIDEO_ELEMENTS.with_borrow_mut(|ve| {
            if let Some(video_element) = ve.get(&resize_message.0) {
                // This probably doesn't work if the video texture resizes while playing
                // The material would need change detection triggered to refresh the new texture
                // https://github.com/bevyengine/bevy/issues/16159
                images.insert(
                    resize_message.0,
                    new_image(Extent3d {
                        width: video_element.element.video_width(),
                        height: video_element.element.video_height(),
                        ..default()
                    }),
                );
                Some(ResizeVideoEvent {
                    asset_id: resize_message.0,
                    width: video_element.element.video_width(),
                    height: video_element.element.video_height(),
                })
            } else {
                None
            }
        }) {
            videos
                .iter()
                .filter_map(|(entity, video)| {
                    if video.asset_id == resize_message.0 {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .for_each(|entity| commands.trigger_targets(size_event, entity));
            commands.trigger(size_event);
        }
    }
}

fn render_videos(queue: Res<RenderQueue>, images: Res<RenderAssets<GpuImage>>) {
    VIDEO_ELEMENTS.with_borrow(|ve| {
        ve.iter()
            .filter_map(|(asset_id, video_element)| {
                if video_element.loaded
                    && let Some(gpu_image) = images.get(*asset_id)
                {
                    Some((gpu_image, video_element))
                } else {
                    None
                }
            })
            .for_each(|(gpu_image, video_element)| {
                queue.copy_external_image_to_texture(
                    &CopyExternalImageSourceInfo {
                        source: ExternalImageSource::HTMLVideoElement(
                            video_element.element.clone(),
                        ),
                        origin: Origin2d::ZERO,
                        flip_y: false,
                    },
                    CopyExternalImageDestInfo {
                        texture: &gpu_image.texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                        color_space: PredefinedColorSpace::Srgb,
                        premultiplied_alpha: true,
                    },
                    gpu_image.size,
                );
            });
    });
}
