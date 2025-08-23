use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use event::{LoadedMetadata, Playing, Resize};
use wasm_bindgen::prelude::*;

pub mod event;
mod listener;
mod render;
use listener::AddEventListenerExt;
pub use listener::{EntityAddEventListenerExt, EventListenerApp, ListenerEvent};

use crate::event::Error;

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_listener_event::<LoadedMetadata>()
            .add_listener_event::<Resize>()
            .add_listener_event::<Playing>()
            .add_listener_event::<Error>()
            .add_systems(Update, (remove_unused_video_elements, handle_new_videos));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            render::render_videos.in_set(RenderSet::PrepareResources),
        );
    }
}

// wasm on web is single threaded, so this should be OK
thread_local! {
    static VIDEO_ELEMENTS: RefCell<HashMap<VideoId, VideoElement>> =  RefCell::new(HashMap::default());
}

struct VideoElement {
    element: web_sys::HtmlVideoElement,
    loaded: bool,
    text_track: Option<web_sys::TextTrack>,
    listeners: Vec<EventListener>,
}

pub trait AssetsVideoIdExt {
    fn new_video_texture(&mut self) -> (Handle<Image>, VideoId, web_sys::HtmlVideoElement);
}

impl AssetsVideoIdExt for Assets<Image> {
    fn new_video_texture(&mut self) -> (Handle<Image>, VideoId, web_sys::HtmlVideoElement) {
        let image_handle = self.get_handle_provider().reserve_handle().typed::<Image>();
        let video_id = VideoId(image_handle.id());

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

        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            elements.insert(
                video_id,
                VideoElement {
                    element: html_video_element.clone(),
                    loaded: false,
                    text_track: None,
                    listeners: Vec::new(),
                },
            )
        });
        (image_handle, video_id, html_video_element)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct VideoId(AssetId<Image>);

#[derive(Clone, Component)]
pub struct WebVideo(VideoId);

impl WebVideo {
    pub fn new(video_id: VideoId) -> Self {
        Self(video_id)
    }

    fn video_id(&self) -> VideoId {
        self.0
    }

    pub fn video_element(&self) -> web_sys::HtmlVideoElement {
        let video_id = self.0;
        VIDEO_ELEMENTS.with_borrow(|elements| {
            elements
                .get(&video_id)
                .expect_throw("Missing video element")
                .element
                .clone()
        })
    }
}

fn remove_unused_video_elements(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Image>>,
    web_videos: Query<(Entity, &WebVideo)>,
) {
    for event in events.read() {
        if let AssetEvent::Unused { id: asset_id } = event {
            let video_id = VideoId(*asset_id);
            if VIDEO_ELEMENTS
                .with_borrow_mut(|elements| elements.remove(&video_id))
                .is_some()
            {
                web_videos
                    .iter()
                    .filter_map(|(entity, web_video)| {
                        if web_video.video_id() == video_id {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                    .for_each(|entity| {
                        commands.entity(entity).remove::<WebVideo>();
                    });
            }
        }
    }
}

fn handle_new_videos(
    mut commands: Commands,
    web_videos: Query<&WebVideo, Added<WebVideo>>,
    mut images: ResMut<Assets<Image>>,
) {
    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
        for web_video in &web_videos {
            let video_id = web_video.video_id();
            if let Some(video_element) = elements.get_mut(&video_id) {
                let ready_state = video_element.element.ready_state();
                if ready_state >= web_sys::HtmlMediaElement::HAVE_METADATA {
                    images.insert(
                        video_id.0,
                        new_image(Extent3d {
                            width: video_element.element.video_width(),
                            height: video_element.element.video_height(),
                            depth_or_array_layers: 1,
                        }),
                    );
                } else {
                    commands.add_event_listener(video_id, on_loaded_metadata);
                };
                commands.add_event_listener(video_id, on_resize);
                commands.add_event_listener(video_id, on_error);
                if !video_element.element.paused()
                    && !video_element.element.ended()
                    && ready_state >= web_sys::HtmlMediaElement::HAVE_CURRENT_DATA
                    && video_element.element.current_time() > 0.0
                {
                    info!("video already playing {video_id:?}"); //XXX
                    video_element.loaded = true;
                } else {
                    commands.add_event_listener(video_id, on_playing);
                }
            }
        }
    });
}

fn handle_resize(video_id: VideoId, images: &mut Assets<Image>) {
    VIDEO_ELEMENTS.with_borrow(|elements| {
        if let Some(video_element) = elements.get(&video_id) {
            images.insert(
                video_id.0,
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
    handle_resize(trigger.video_id(), &mut images);
}

fn on_error(trigger: Trigger<ListenerEvent<Error>>) {
    let video_id = trigger.video_id();
    warn!("Video {video_id:?} failed with error");
}

fn on_resize(trigger: Trigger<ListenerEvent<Resize>>, mut images: ResMut<Assets<Image>>) {
    handle_resize(trigger.video_id(), &mut images);
}

fn on_playing(trigger: Trigger<ListenerEvent<Playing>>) {
    let video_id = trigger.video_id();
    info!("on_playing for {video_id:?}"); //XXX
    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
        if let Some(video_element) = elements.get_mut(&video_id) {
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
