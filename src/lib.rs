use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

use bevy::{
    asset::RenderAssetUsages,
    ecs::world::CommandQueue,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use event::{LoadedMetadata, Playing, Resize};
use wasm_bindgen::prelude::*;

mod commands;
pub mod event;
mod listener;
mod render;
use listener::CommandsAddEventListenerExt;
pub use listener::{EntityAddVideoEventListenerExt, EventListenerApp, ListenerEvent};

use crate::{event::Error, listener::ListenerCommand};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        let rx = VIDEO_ELEMENTS.with_borrow(|elements| elements.rx.clone());
        app.insert_resource(CommandReceiver(rx))
            .add_listener_event::<LoadedMetadata>()
            .add_listener_event::<Resize>()
            .add_listener_event::<Playing>()
            .add_listener_event::<Error>()
            .add_systems(Update, (despawn_unused_videos, handle_commands));
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
    static VIDEO_ELEMENTS: RefCell<VideoElements> =  RefCell::new(VideoElements::new());
}

struct VideoElements {
    tx: crossbeam_channel::Sender<ListenerCommand>,
    rx: crossbeam_channel::Receiver<ListenerCommand>,
    elements: HashMap<VideoId, VideoElement>,
}

impl VideoElements {
    fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            tx,
            rx,
            elements: HashMap::default(),
        }
    }
}

struct VideoElement {
    element: web_sys::HtmlVideoElement,
    loaded: bool,
    text_track: Option<web_sys::TextTrack>,
    listeners: Vec<EventListener>,
}

impl VideoElement {
    fn add_event_listener(
        &mut self,
        event_name: &'static str,
        tx: crossbeam_channel::Sender<ListenerCommand>,
        command: ListenerCommand,
    ) {
        let callback = move |_event: &web_sys::Event| {
            if let Err(err) = tx.send(command.clone()) {
                warn!("Failed to register listener: {err:?}");
            };
        };
        let listener = EventListener::new(&self.element, event_name, callback);
        self.listeners.push(listener);
    }
}

#[derive(Resource)]
struct CommandReceiver(crossbeam_channel::Receiver<ListenerCommand>);

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct VideoId(AssetId<Image>);

#[derive(Clone, Component)]
pub struct WebVideo(VideoId);

impl WebVideo {
    pub(crate) fn new(video_id: VideoId) -> Self {
        Self(video_id)
    }

    fn video_id(&self) -> VideoId {
        self.0
    }
}

fn handle_commands(mut commands: Commands, receiver: Res<CommandReceiver>) {
    while let Ok(command) = receiver.0.try_recv() {
        commands.queue(command);
    }
}

fn despawn_unused_videos(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Image>>,
    web_videos: Query<(Entity, &WebVideo)>,
) {
    for event in events.read() {
        if let AssetEvent::Unused { id: asset_id } = event {
            let video_id = VideoId(*asset_id);
            if VIDEO_ELEMENTS
                .with_borrow_mut(|elements| elements.elements.remove(&video_id))
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
                        commands.entity(entity).despawn();
                    });
            }
        }
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
