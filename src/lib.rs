use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
};
use wasm_bindgen::prelude::*;

mod asset;
mod event;
mod extensions;
mod registry;
mod render;

pub use crate::{
    asset::VideoSource,
    event::{EventListenerApp, EventType, ListenerEvent, events},
    extensions::{
        AddVideoTextureExt, EntityAddVideoEventListenerExt, EntityCommandsWithVideoElementExt,
    },
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(asset::plugin)
            .add_listener_event::<events::LoadedMetadata>()
            .add_listener_event::<events::Resize>()
            .add_listener_event::<events::Playing>()
            .add_listener_event::<events::Error>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            render::render_videos.in_set(RenderSet::PrepareResources),
        );
    }
}

#[derive(Clone, Component)]
pub struct WebVideo(Handle<VideoSource>);

impl WebVideo {
    pub fn new(source: Handle<VideoSource>) -> Self {
        Self(source)
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
