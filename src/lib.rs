use crossbeam_channel::unbounded;
use enumset::EnumSet;
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

use crate::event::{LoadedMetadata, Playing, Resize, VideoEvent, VideoEventMessage, VideoEvents};

pub mod event;
pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = unbounded();
        app.insert_resource(WebVideoRegistry { rx, tx })
            .add_systems(Update, (remove_unused_video_elements, trigger_video_events))
            .add_observer(observe_loaded_metadata)
            .add_observer(observe_resize)
            .add_observer(observe_playing);
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
    enabled_events: EnumSet<VideoEvents>,
}

#[derive(Resource)]
pub struct WebVideoRegistry {
    rx: crossbeam_channel::Receiver<VideoEventMessage>,
    tx: crossbeam_channel::Sender<VideoEventMessage>,
}

#[derive(Clone, Component)]
pub struct WebVideo {
    asset_id: AssetId<Image>,
}

#[derive(Debug)]
pub struct WebVideoError {
    message: String,
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

        VIDEO_ELEMENTS.with_borrow_mut(|ve| {
            ve.insert(
                asset_id,
                VideoElement {
                    element: html_video_element.clone(),
                    loaded: false,
                    enabled_events: EnumSet::empty(),
                },
            )
        });

        self.enable_observer(VideoEvents::Resize, asset_id)?;
        self.enable_observer(VideoEvents::LoadedMetadata, asset_id)?;
        self.enable_observer(VideoEvents::Playing, asset_id)?;

        Ok((image_handle, html_video_element))
    }

    pub fn enable_observer(&self, event_type: VideoEvents, asset_id: AssetId<Image>) -> Result<()> {
        let tx = self.tx.clone();
        VIDEO_ELEMENTS.with_borrow_mut(|ve| {
            if let Some(video_element) = ve.get_mut(&asset_id)
                && !video_element.enabled_events.contains(event_type)
            {
                let callback = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::Event| {
                    if let Err(err) = tx.send(VideoEventMessage {
                        asset_id,
                        event_type,
                    }) {
                        warn!("Failed to handle video {event_type:?}: {err:?}");
                    }
                });
                video_element
                    .element
                    .add_event_listener_with_callback(
                        event_type.into(),
                        callback.as_ref().unchecked_ref(),
                    )
                    .map_err(WebVideoError::from)?;
                callback.forget();
                video_element.enabled_events.insert(event_type);
            }
            Ok(())
        })
    }
}

impl WebVideo {
    pub fn asset_id(&self) -> AssetId<Image> {
        self.asset_id
    }

    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        let asset_id = self.asset_id;
        VIDEO_ELEMENTS.with_borrow(|ve| ve.get(&asset_id).map(|e| e.element.clone()))
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

fn dispatch_event<E>(
    event: VideoEvent<E>,
    commands: &mut Commands,
    videos: Query<(Entity, &WebVideo)>,
) where
    E: std::fmt::Debug + Copy + Clone + Send + Sync + 'static,
{
    videos
        .iter()
        .filter_map(|(entity, video)| {
            if video.asset_id == event.asset_id {
                Some(entity)
            } else {
                None
            }
        })
        .for_each(|entity| commands.trigger_targets(event, entity));
    commands.trigger(event);
}

fn trigger_video_events(
    mut commands: Commands,
    video_registry: Res<WebVideoRegistry>,
    videos: Query<(Entity, &WebVideo)>,
) {
    while let Ok(VideoEventMessage {
        asset_id,
        event_type,
    }) = video_registry.rx.try_recv()
    {
        VIDEO_ELEMENTS.with_borrow(|ve| {
            if let Some(video_element) = ve.get(&asset_id) {
                match event_type {
                    VideoEvents::Resize => dispatch_event(
                        VideoEvent {
                            asset_id,
                            event: Resize {
                                width: video_element.element.video_width(),
                                height: video_element.element.video_height(),
                            },
                        },
                        &mut commands,
                        videos,
                    ),
                    VideoEvents::LoadedMetadata => dispatch_event(
                        VideoEvent {
                            asset_id,
                            event: LoadedMetadata {
                                width: video_element.element.video_width(),
                                height: video_element.element.video_height(),
                            },
                        },
                        &mut commands,
                        videos,
                    ),
                    VideoEvents::Playing => dispatch_event(
                        VideoEvent {
                            asset_id,
                            event: Playing,
                        },
                        &mut commands,
                        videos,
                    ),
                };
            }
        });
    }
}

fn handle_video_size(
    width: u32,
    height: u32,
    asset_id: AssetId<Image>,
    mut images: ResMut<Assets<Image>>,
) {
    images.insert(
        asset_id,
        new_image(Extent3d {
            width,
            height,
            ..default()
        }),
    );
}

fn observe_loaded_metadata(
    trigger: Trigger<VideoEvent<LoadedMetadata>>,
    images: ResMut<Assets<Image>>,
) {
    let VideoEvent {
        asset_id,
        event: LoadedMetadata { width, height },
    } = trigger.event();
    handle_video_size(*width, *height, *asset_id, images);
}

fn observe_resize(trigger: Trigger<VideoEvent<Resize>>, images: ResMut<Assets<Image>>) {
    // This probably doesn't work if the video texture resizes while playing
    // The material would need change detection triggered to refresh the new texture
    // https://github.com/bevyengine/bevy/issues/16159
    let VideoEvent {
        asset_id,
        event: Resize { width, height },
    } = trigger.event();
    handle_video_size(*width, *height, *asset_id, images)
}

fn observe_playing(trigger: Trigger<VideoEvent<Playing>>) {
    let asset_id = trigger.event().asset_id;
    VIDEO_ELEMENTS.with_borrow_mut(|ve| {
        if let Some(video_element) = ve.get_mut(&asset_id) {
            video_element.loaded = true;
        }
    });
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
