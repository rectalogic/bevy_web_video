use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
};
use event::{LoadedMetadata, Playing, Resize};
use wasm_bindgen::prelude::*;

mod asset;
mod commands;
pub mod event;
mod listener;
mod render;
pub use listener::{EntityAddVideoEventListenerExt, ListenerEvent};

use crate::{
    asset::{Registry, VideoSource},
    event::Error,
    listener::ListenerCommand,
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        let rx = Registry::with_borrow(|registry| registry.receiver());
        app.init_asset::<VideoSource>()
            .insert_resource(CommandReceiver(rx))
            .add_event::<ListenerEvent<LoadedMetadata>>()
            .add_event::<ListenerEvent<Resize>>()
            .add_event::<ListenerEvent<Playing>>()
            .add_event::<ListenerEvent<Error>>()
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
