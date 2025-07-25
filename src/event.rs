use bevy::prelude::*;
use enumset::EnumSetType;

use crate::WebVideo;

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

pub(crate) fn dispatch_events(
    event_type: VideoEvents,
    asset_id: AssetId<Image>,
    video: &web_sys::HtmlVideoElement,
    commands: &mut Commands,
    videos: Query<(Entity, &WebVideo)>,
) {
    match event_type {
        VideoEvents::Abort => trigger_event(asset_id, Abort, commands, videos),
        VideoEvents::CanPlay => trigger_event(asset_id, CanPlay, commands, videos),
        VideoEvents::CanPlayThrough => trigger_event(asset_id, CanPlayThrough, commands, videos),
        VideoEvents::DurationChanged => trigger_event(asset_id, DurationChanged, commands, videos),
        VideoEvents::Emptied => trigger_event(asset_id, Emptied, commands, videos),
        VideoEvents::Ended => trigger_event(asset_id, Ended, commands, videos),
        VideoEvents::Error => trigger_event(asset_id, Error, commands, videos),
        VideoEvents::LoadedData => trigger_event(asset_id, LoadedData, commands, videos),
        VideoEvents::LoadedMetadata => trigger_event(
            asset_id,
            LoadedMetadata {
                width: video.video_width(),
                height: video.video_height(),
            },
            commands,
            videos,
        ),
        VideoEvents::LoadStart => trigger_event(asset_id, LoadStart, commands, videos),
        VideoEvents::Pause => trigger_event(asset_id, Pause, commands, videos),
        VideoEvents::Play => trigger_event(asset_id, Play, commands, videos),
        VideoEvents::Playing => trigger_event(asset_id, Playing, commands, videos),
        VideoEvents::Progress => trigger_event(asset_id, Progress, commands, videos),
        VideoEvents::RateChange => trigger_event(asset_id, RateChange, commands, videos),
        VideoEvents::Resize => trigger_event(
            asset_id,
            Resize {
                width: video.video_width(),
                height: video.video_height(),
            },
            commands,
            videos,
        ),
        VideoEvents::Seeked => trigger_event(asset_id, Seeked, commands, videos),
        VideoEvents::Seeking => trigger_event(asset_id, Seeking, commands, videos),
        VideoEvents::Stalled => trigger_event(asset_id, Stalled, commands, videos),
        VideoEvents::Suspend => trigger_event(asset_id, Suspend, commands, videos),
        VideoEvents::TimeUpdate => trigger_event(asset_id, TimeUpdate, commands, videos),
        VideoEvents::VolumeChange => trigger_event(asset_id, VolumeChange, commands, videos),
        VideoEvents::Waiting => trigger_event(asset_id, Waiting, commands, videos),
        VideoEvents::WaitingForKey => trigger_event(asset_id, WaitingForKey, commands, videos),
    };
}

fn trigger_event<E>(
    asset_id: AssetId<Image>,
    event: E,
    commands: &mut Commands,
    videos: Query<(Entity, &WebVideo)>,
) where
    E: std::fmt::Debug + Copy + Clone + Send + Sync + 'static,
{
    let video_event = VideoEvent { asset_id, event };
    videos
        .iter()
        .filter_map(|(entity, video)| {
            if video.0 == asset_id {
                Some(entity)
            } else {
                None
            }
        })
        .for_each(|entity| commands.trigger_targets(video_event, entity));
    commands.trigger(video_event);
}
