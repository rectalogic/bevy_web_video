use bevy::prelude::*;
use enumset::EnumSetType;

pub struct VideoEvent {
    pub asset_id: AssetId<Image>,
    pub event_type: VideoEventType,
}

#[derive(EnumSetType, Debug)]
pub enum VideoEventType {
    Resize,
    LoadedMetadata,
    Playing,
}

pub(crate) trait VideoEventAsset: Event<Traversal = ()> + Copy + std::fmt::Debug {
    fn asset_id(&self) -> AssetId<Image>;
}

#[derive(Copy, Clone, Debug, Event)]
pub struct Resize(pub AssetId<Image>);
impl VideoEventAsset for Resize {
    fn asset_id(&self) -> AssetId<Image> {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Event)]
pub struct LoadedMetadata(pub AssetId<Image>);
impl VideoEventAsset for LoadedMetadata {
    fn asset_id(&self) -> AssetId<Image> {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Event)]
pub struct Playing(pub AssetId<Image>);
impl VideoEventAsset for Playing {
    fn asset_id(&self) -> AssetId<Image> {
        self.0
    }
}

impl From<VideoEventType> for &'static str {
    fn from(value: VideoEventType) -> Self {
        match value {
            VideoEventType::Resize => "resize",
            VideoEventType::LoadedMetadata => "loadedmetadata",
            VideoEventType::Playing => "playing",
        }
    }
}
