use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

use bevy::{
    asset::{AsAssetId, RenderAssetUsages},
    ecs::{component::HookContext, world::DeferredWorld},
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

mod event;
mod listener;
pub use event::{LoadedMetadata, Playing, Resize};
pub use listener::{
    AddEventListenerExt, EntityAddEventListenerExt, EventListenerApp, ListenerEvent,
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_listener_event::<LoadedMetadata>()
            .add_listener_event::<Resize>()
            .add_listener_event::<Playing>()
            .add_systems(Update, handle_new_videos);
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

struct VideoElement {
    element: web_sys::HtmlVideoElement,
    loaded: bool,
    text_track: Option<web_sys::TextTrack>,
    listeners: Vec<EventListener>,
}

#[derive(Clone, Component)]
#[component(on_remove = on_remove_web_video)]
pub struct WebVideo(Handle<Image>);

impl WebVideo {
    pub fn new(images: &Assets<Image>) -> Self {
        let image = images
            .get_handle_provider()
            .reserve_handle()
            .typed::<Image>();
        let html_video_element = web_sys::window()
            .expect_throw("window")
            .document()
            .expect_throw("document")
            .create_element("video")
            .inspect_err(|e| warn!("{e:?}"))
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlVideoElement>()
            .inspect_err(|e| warn!("{e:?}"))
            .expect_throw("web_sys::HtmlVideoElement");

        let asset_id = image.id();

        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            elements.insert(
                asset_id,
                VideoElement {
                    element: html_video_element,
                    loaded: false,
                    text_track: None,
                    listeners: Vec::new(),
                },
            )
        });

        Self(image)
    }

    pub fn video_element(&self) -> web_sys::HtmlVideoElement {
        let asset_id = self.0.id();
        VIDEO_ELEMENTS.with_borrow(|elements| {
            elements
                .get(&asset_id)
                .expect_throw("Missing video element")
                .element
                .clone()
        })
    }
}

fn on_remove_web_video(world: DeferredWorld, context: HookContext) {
    let asset_id = world.get::<WebVideo>(context.entity).unwrap().0.id();
    VIDEO_ELEMENTS.with_borrow_mut(|elements| elements.remove(&asset_id));
}

impl AsAssetId for WebVideo {
    type Asset = Image;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

fn handle_new_videos(
    mut commands: Commands,
    web_videos: Query<&WebVideo, Added<WebVideo>>,
    mut images: ResMut<Assets<Image>>,
) {
    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
        for web_video in &web_videos {
            let asset_id = web_video.as_asset_id();
            if let Some(video_element) = elements.get_mut(&asset_id) {
                let ready_state = video_element.element.ready_state();
                if ready_state >= web_sys::HtmlMediaElement::HAVE_METADATA {
                    images.insert(
                        asset_id,
                        new_image(Extent3d {
                            width: video_element.element.video_width(),
                            height: video_element.element.video_height(),
                            depth_or_array_layers: 1,
                        }),
                    );
                } else {
                    commands.add_event_listener(web_video, on_loaded_metadata);
                };
                commands.add_event_listener(web_video, on_resize);
                if !video_element.element.paused()
                    && !video_element.element.ended()
                    && ready_state >= web_sys::HtmlMediaElement::HAVE_CURRENT_DATA
                    && video_element.element.current_time() > 0.0
                {
                    video_element.loaded = true;
                } else {
                    commands.add_event_listener(web_video, on_playing);
                }
            }
        }
    });
}

fn handle_resize(asset_id: AssetId<Image>, images: &mut Assets<Image>) {
    VIDEO_ELEMENTS.with_borrow(|elements| {
        if let Some(video_element) = elements.get(&asset_id) {
            images.insert(
                asset_id,
                new_image(Extent3d {
                    width: video_element.element.video_width(),
                    height: video_element.element.video_height(),
                    depth_or_array_layers: 1,
                }),
            );
        }
    });
}

fn on_loaded_metadata(
    trigger: Trigger<ListenerEvent<LoadedMetadata>>,
    mut images: ResMut<Assets<Image>>,
) {
    handle_resize(trigger.asset_id(), &mut images);
}

fn on_resize(trigger: Trigger<ListenerEvent<Resize>>, mut images: ResMut<Assets<Image>>) {
    handle_resize(trigger.asset_id(), &mut images);
}

fn on_playing(trigger: Trigger<ListenerEvent<Playing>>) {
    let asset_id = trigger.asset_id();
    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
        if let Some(video_element) = elements.get_mut(&asset_id) {
            video_element.loaded = true;
        }
    });
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

#[derive(Debug)]
pub struct WebVideoError {
    message: String,
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
