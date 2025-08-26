use crate::event::{EventType, ListenerEvent};
use bevy::prelude::*;
use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

// wasm on web is single threaded, so this should be OK
thread_local! {
    static REGISTRY: RefCell<ElementRegistry> =  RefCell::new(ElementRegistry::new());
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

pub struct ElementRegistry {
    pub next_id: RegistryId,
    pub elements: HashMap<RegistryId, RegisteredElement>,
}

impl ElementRegistry {
    pub fn new() -> Self {
        Self {
            next_id: RegistryId::new(),
            elements: HashMap::default(),
        }
    }

    pub fn allocate_id(&mut self) -> RegistryId {
        let id = self.next_id;
        self.next_id.increment();
        id
    }

    pub fn add(&mut self, element: RegisteredElement) -> RegistryId {
        let id = self.allocate_id();
        self.insert(id, element);
        id
    }

    pub fn insert(&mut self, registry_id: RegistryId, element: RegisteredElement) {
        self.elements.insert(registry_id, element);
    }

    pub fn remove(&mut self, registry_id: RegistryId) -> Option<RegisteredElement> {
        self.elements.remove(&registry_id)
    }

    pub fn get(&self, registry_id: &RegistryId) -> Option<&RegisteredElement> {
        self.elements.get(registry_id)
    }

    pub fn get_mut(&mut self, registry_id: &RegistryId) -> Option<&mut RegisteredElement> {
        self.elements.get_mut(registry_id)
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

pub struct RegisteredElement {
    element: web_sys::HtmlVideoElement,
    listeners: Vec<EventListener>,
}

impl RegisteredElement {
    pub fn new(element: web_sys::HtmlVideoElement) -> Self {
        Self {
            element,
            listeners: Vec::default(),
        }
    }

    pub fn element(&self) -> &web_sys::HtmlVideoElement {
        &self.element
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
