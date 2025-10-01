use bevy::{asset::AsAssetId, prelude::*};
use wasm_bindgen::prelude::*;

mod event;
mod registry;
pub(crate) mod render;

pub use crate::{
    event::{EventListenerAppExt, EventSender, EventType, ListenerEvent, events},
    registry::{
        VideoElementRegistry,
        asset::{VideoElement, VideoElementAssetsExt},
    },
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        // event must be built before registry
        app.add_plugins((event::plugin, registry::plugin, render::VideoRenderPlugin));
    }
}

#[derive(Clone, Component)]
pub struct WebVideo(Handle<VideoElement>);

impl WebVideo {
    pub fn new(video_element: Handle<VideoElement>) -> Self {
        Self(video_element)
    }

    pub fn asset_id(&self) -> AssetId<VideoElement> {
        self.0.id()
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
