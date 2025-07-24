use bevy::prelude::*;
use enumset::EnumSetType;

pub(crate) struct VideoEventMessage {
    pub asset_id: AssetId<Image>,
    pub event_type: VideoEvents,
}

#[derive(EnumSetType, Debug)]
pub enum VideoEvents {
    Resize,
    LoadedMetadata,
    Playing,
}

#[derive(Copy, Clone, Debug, Event)]
pub struct VideoEvent<E>
where
    E: std::fmt::Debug + Copy + Clone + Send + Sync,
{
    pub asset_id: AssetId<Image>,
    pub event: E,
}

#[derive(Copy, Clone, Debug)]
pub struct Resize {
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct LoadedMetadata {
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Playing;

impl From<VideoEvents> for &'static str {
    fn from(value: VideoEvents) -> Self {
        match value {
            VideoEvents::Resize => "resize",
            VideoEvents::LoadedMetadata => "loadedmetadata",
            VideoEvents::Playing => "playing",
        }
    }
}
