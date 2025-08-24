use std::marker::PhantomData;

use bevy::prelude::*;

use crate::registry::{Registry, RegistryId};

#[derive(Event)]
pub struct ListenerEvent<E: EventType> {
    registry_id: RegistryId,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    pub(crate) fn new(registry_id: RegistryId, target: Option<Entity>) -> Self {
        Self {
            registry_id,
            target,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        Registry::with_borrow(|registry| registry.get(&self.registry_id).map(|e| e.element.clone()))
    }
}

pub trait EventType: Sized + Send + Sync + 'static {
    const EVENT_NAME: &'static str;
}

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
new_event_type!(Error, "error");
