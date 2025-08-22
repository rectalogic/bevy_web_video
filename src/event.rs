use bevy::prelude::*;

use crate::listener::EventType;

#[derive(Event, Copy, Clone, Debug)]
pub struct LoadedMetadata;
impl EventType for LoadedMetadata {
    const EVENT_NAME: &'static str = "loadedmetadata";
    fn new() -> Self {
        Self
    }
}

#[derive(Event, Copy, Clone, Debug)]
pub struct Resize;
impl EventType for Resize {
    const EVENT_NAME: &'static str = "resize";
    fn new() -> Self {
        Self
    }
}

#[derive(Event, Copy, Clone, Debug)]
pub struct Playing;
impl EventType for Playing {
    const EVENT_NAME: &'static str = "playing";
    fn new() -> Self {
        Self
    }
}
