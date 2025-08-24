use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
};
use wasm_bindgen::prelude::*;

mod asset;
mod commands;
pub mod event;
mod listener;
mod registry;
mod render;

pub use crate::{
    asset::{AddVideoTextureExt, VideoSource},
    commands::EntityCommandsWithVideoElementExt,
    listener::EntityAddVideoEventListenerExt,
};
use crate::{listener::ListenerCommand, registry::Registry};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        let rx = Registry::with_borrow(|registry| registry.receiver());
        app.init_asset::<VideoSource>()
            .insert_resource(CommandReceiver(rx))
            .add_event::<event::ListenerEvent<event::LoadedMetadata>>()
            .add_event::<event::ListenerEvent<event::Resize>>()
            .add_event::<event::ListenerEvent<event::Playing>>()
            .add_event::<event::ListenerEvent<event::Error>>()
            .add_systems(Update, handle_commands);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            render::render_videos.in_set(RenderSet::PrepareResources),
        );
    }
}

#[derive(Resource)]
struct CommandReceiver(crossbeam_channel::Receiver<ListenerCommand>);

#[derive(Clone, Component)]
pub struct WebVideo(Handle<VideoSource>);

impl WebVideo {
    pub fn new(source: Handle<VideoSource>) -> Self {
        Self(source)
    }
}

fn handle_commands(mut commands: Commands, receiver: Res<CommandReceiver>) {
    while let Ok(command) = receiver.0.try_recv() {
        commands.queue(command);
    }
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
