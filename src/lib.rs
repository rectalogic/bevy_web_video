use crossbeam_channel::unbounded;
use enumset::EnumSet;
use std::{cell::RefCell, collections::HashMap};

use bevy::{
    asset::{AsAssetId, RenderAssetUsages},
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

use crate::event::{VideoEvent, VideoEventMessage, VideoEvents};

pub mod event;
mod listener;
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
    text_track: Option<web_sys::TextTrack>,
    enabled_events: EnumSet<VideoEvents>,
}

#[derive(Resource)]
pub struct WebVideoRegistry {
    rx: crossbeam_channel::Receiver<VideoEventMessage>,
    tx: crossbeam_channel::Sender<VideoEventMessage>,
}

#[derive(Clone, Component)]
pub struct WebVideo(pub AssetId<Image>);

//XXX add a component hook so when this is removed we remove from registry (and dump Asset Unused stuff)
#[derive(Clone, Component)]
pub struct WebVideoSink(Handle<Image>);

impl WebVideoSink {
    fn new(image: Handle<Image>) -> Self {
        Self(image)
    }
}
impl AsAssetId for WebVideoSink {
    type Asset = Image;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

#[derive(Debug)]
pub struct WebVideoError {
    message: String,
}

impl WebVideoRegistry {
    pub fn new_video_texture(
        &self,
        images: &Assets<Image>,
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
                    text_track: None,
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
                if event_type == VideoEvents::CueChange {
                    let track = video_element.text_track.get_or_insert_with(|| {
                        video_element
                            .element
                            .add_text_track(web_sys::TextTrackKind::Metadata)
                    });
                    track
                        .add_event_listener_with_callback(
                            event_type.into(),
                            callback.as_ref().unchecked_ref(),
                        )
                        .map_err(WebVideoError::from)?;
                } else {
                    video_element
                        .element
                        .add_event_listener_with_callback(
                            event_type.into(),
                            callback.as_ref().unchecked_ref(),
                        )
                        .map_err(WebVideoError::from)?;
                }
                callback.forget();
                video_element.enabled_events.insert(event_type);
            }
            Ok(())
        })
    }

    pub fn add_cue(&self, cue: event::Cue, asset_id: AssetId<Image>) -> Result<()> {
        self.enable_observer(VideoEvents::CueChange, asset_id)?;
        VIDEO_ELEMENTS.with_borrow(|ve| {
            if let Some(video_element) = ve.get(&asset_id)
                && let Some(ref text_track) = video_element.text_track
            {
                text_track.add_cue(
                    &web_sys::VttCue::new(cue.start_time, cue.end_time, "")
                        .map_err(WebVideoError::from)?,
                );
            }
            Ok(())
        })
    }
}

impl WebVideo {
    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        let asset_id = self.0;
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
                event::dispatch_events(event_type, asset_id, video_element, &mut commands, videos);
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
    trigger: Trigger<VideoEvent<event::LoadedMetadata>>,
    images: ResMut<Assets<Image>>,
) {
    let VideoEvent {
        asset_id,
        event: event::LoadedMetadata { width, height },
    } = trigger.event();
    handle_video_size(*width, *height, *asset_id, images);
}

fn observe_resize(trigger: Trigger<VideoEvent<event::Resize>>, images: ResMut<Assets<Image>>) {
    // This probably doesn't work if the video texture resizes while playing
    // The material would need change detection triggered to refresh the new texture
    // https://github.com/bevyengine/bevy/issues/16159
    let VideoEvent {
        asset_id,
        event: event::Resize { width, height },
    } = trigger.event();
    handle_video_size(*width, *height, *asset_id, images)
}

// copy_external_image_to_texture too early results in panic on Chrome:
// https://github.com/gfx-rs/wgpu/issues/8005
fn observe_playing(trigger: Trigger<VideoEvent<event::Playing>>) {
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
