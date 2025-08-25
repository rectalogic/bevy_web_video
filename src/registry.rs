use crate::event::{EventType, ListenerEvent};
use bevy::prelude::*;
use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

// wasm on web is single threaded, so this should be OK
thread_local! {
    static REGISTRY: RefCell<Registry> =  RefCell::new(Registry::new());
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegistryId(u64);

impl RegistryId {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn increment(&mut self) {
        self.0 += 1;
    }
}

pub struct Registry {
    pub(crate) next_id: RegistryId,
    pub(crate) elements: HashMap<RegistryId, VideoElement>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            next_id: RegistryId::new(),
            elements: HashMap::default(),
        }
    }

    pub(crate) fn allocate_id(&mut self) -> RegistryId {
        let id = self.next_id;
        self.next_id.increment();
        id
    }

    pub(crate) fn add(&mut self, video_element: VideoElement) -> RegistryId {
        let id = self.allocate_id();
        self.insert(id, video_element);
        id
    }

    pub(crate) fn insert(&mut self, registry_id: RegistryId, video_element: VideoElement) {
        self.elements.insert(registry_id, video_element);
    }

    pub(crate) fn remove(&mut self, registry_id: RegistryId) -> Option<VideoElement> {
        self.elements.remove(&registry_id)
    }

    pub fn get(&self, registry_id: &RegistryId) -> Option<&VideoElement> {
        self.elements.get(registry_id)
    }

    pub fn get_mut(&mut self, registry_id: &RegistryId) -> Option<&mut VideoElement> {
        self.elements.get_mut(registry_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VideoElement> {
        self.elements.values()
    }

    pub fn with_borrow<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        REGISTRY.with_borrow(f)
    }

    pub fn with_borrow_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        REGISTRY.with_borrow_mut(f)
    }
}

pub struct VideoElement {
    target_texture_id: AssetId<Image>,
    element: web_sys::HtmlVideoElement,
    loaded: bool,
    listeners: Vec<EventListener>,
}

impl VideoElement {
    pub fn new(target_texture_id: AssetId<Image>, element: web_sys::HtmlVideoElement) -> Self {
        Self {
            target_texture_id,
            element,
            loaded: false,
            listeners: Vec::default(),
        }
    }

    pub fn element(&self) -> web_sys::HtmlVideoElement {
        self.element.clone()
    }

    pub fn target_texture_id(&self) -> AssetId<Image> {
        self.target_texture_id
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_loaded(&mut self) {
        self.loaded = true;
    }

    pub fn add_event_listener<E: EventType>(
        &mut self,
        event: ListenerEvent<E>,
        tx: crossbeam_channel::Sender<ListenerEvent<E>>,
    ) {
        let listener = EventListener::new(
            &self.element,
            E::EVENT_NAME,
            move |_event: &web_sys::Event| {
                if let Err(err) = tx.send(event) {
                    warn!("Failed to ad video event listener: {err:?}");
                };
            },
        );
        self.listeners.push(listener);
    }
}
