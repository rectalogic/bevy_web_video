use bevy::{asset::AsAssetId, prelude::*};
use wasm_bindgen::prelude::*;

mod event;
mod registry;
pub(crate) mod render;

pub use crate::{
    event::{EventListenerAppExt, EventSender, EventType, EventWithAssetId, ListenerEvent, events},
    registry::{
        VideoElementRegistry,
        asset::{VideoElement, VideoElementCreated},
    },
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((registry::plugin, event::plugin, render::VideoRenderPlugin));
    }
}

#[derive(Clone, Component)]
pub struct WebVideo(Handle<VideoElement>);

impl WebVideo {
    pub fn new(video_element: Handle<VideoElement>) -> Self {
        Self(video_element)
    }
}

impl AsAssetId for WebVideo {
    type Asset = VideoElement;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
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
