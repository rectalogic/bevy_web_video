use bevy::prelude::*;
use enumset::EnumSetType;

pub(crate) struct VideoEventMessage {
    pub asset_id: AssetId<Image>,
    pub event_type: VideoEvents,
}

#[derive(EnumSetType, Debug)]
pub enum VideoEvents {
    Abort,
    CanPlay,
    CanPlayThrough,
    DurationChanged,
    Emptied,
    Ended,
    Error,
    LoadedData,
    LoadedMetadata,
    LoadStart,
    Pause,
    Play,
    Playing,
    Progress,
    RateChange,
    Resize,
    Seeked,
    Seeking,
    Stalled,
    Suspend,
    TimeUpdate,
    VolumeChange,
    Waiting,
    WaitingForKey,
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
pub struct Abort;
#[derive(Copy, Clone, Debug)]
pub struct CanPlay;
#[derive(Copy, Clone, Debug)]
pub struct CanPlayThrough;
#[derive(Copy, Clone, Debug)]
pub struct DurationChanged;
#[derive(Copy, Clone, Debug)]
pub struct Emptied;
#[derive(Copy, Clone, Debug)]
pub struct Ended;
#[derive(Copy, Clone, Debug)]
pub struct Error;
#[derive(Copy, Clone, Debug)]
pub struct LoadedData;
#[derive(Copy, Clone, Debug)]
pub struct LoadedMetadata {
    pub width: u32,
    pub height: u32,
}
#[derive(Copy, Clone, Debug)]
pub struct LoadStart;
#[derive(Copy, Clone, Debug)]
pub struct Pause;
#[derive(Copy, Clone, Debug)]
pub struct Play;
#[derive(Copy, Clone, Debug)]
pub struct Playing;
#[derive(Copy, Clone, Debug)]
pub struct Progress;
#[derive(Copy, Clone, Debug)]
pub struct RateChange;
#[derive(Copy, Clone, Debug)]
pub struct Resize {
    pub width: u32,
    pub height: u32,
}
#[derive(Copy, Clone, Debug)]
pub struct Seeked;
#[derive(Copy, Clone, Debug)]
pub struct Seeking;
#[derive(Copy, Clone, Debug)]
pub struct Stalled;
#[derive(Copy, Clone, Debug)]
pub struct Suspend;
#[derive(Copy, Clone, Debug)]
pub struct TimeUpdate;
#[derive(Copy, Clone, Debug)]
pub struct VolumeChange;
#[derive(Copy, Clone, Debug)]
pub struct Waiting;
#[derive(Copy, Clone, Debug)]
pub struct WaitingForKey;

impl From<VideoEvents> for &'static str {
    fn from(value: VideoEvents) -> Self {
        match value {
            VideoEvents::Abort => "abort",
            VideoEvents::CanPlay => "canplay",
            VideoEvents::CanPlayThrough => "canplaythrough",
            VideoEvents::DurationChanged => "durationchanged",
            VideoEvents::Emptied => "emptied",
            VideoEvents::Ended => "ended",
            VideoEvents::Error => "error",
            VideoEvents::LoadedData => "loadeddata",
            VideoEvents::LoadedMetadata => "loadedmetadata",
            VideoEvents::LoadStart => "loadstart",
            VideoEvents::Pause => "pause",
            VideoEvents::Play => "play",
            VideoEvents::Playing => "playing",
            VideoEvents::Progress => "progress",
            VideoEvents::RateChange => "ratechange",
            VideoEvents::Resize => "resize",
            VideoEvents::Seeked => "seeked",
            VideoEvents::Seeking => "seeking",
            VideoEvents::Stalled => "stalled",
            VideoEvents::Suspend => "suspend",
            VideoEvents::TimeUpdate => "timeupdate",
            VideoEvents::VolumeChange => "volumechange",
            VideoEvents::Waiting => "waiting",
            VideoEvents::WaitingForKey => "waitingforkey",
        }
    }
}
