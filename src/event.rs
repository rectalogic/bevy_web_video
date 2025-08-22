use bevy::prelude::*;

use crate::listener::EventType;

#[macro_export]
macro_rules! new_event_type {
    ($name:ident, $event_name:literal) => {
        #[derive(Event, Copy, Clone, Debug)]
        pub struct $name;

        impl EventType for $name {
            const EVENT_NAME: &'static str = $event_name;
        }
    };
}

new_event_type!(LoadedMetadata, "loadedmetadata");
new_event_type!(Resize, "resize");
new_event_type!(Playing, "playing");
