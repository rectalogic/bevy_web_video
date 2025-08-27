use crate::{
    VideoElement,
    event::{EventType, ListenerEvent},
};
use bevy::prelude::*;
use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap};

pub mod asset;

// wasm on web is single threaded, so this should be OK
thread_local! {
    static REGISTRY: RefCell<ElementRegistry> =  RefCell::new(ElementRegistry::default());
}

#[derive(Default)]
pub struct ElementRegistry {
    pub elements: HashMap<AssetId<VideoElement>, RegisteredElement>,
}

impl ElementRegistry {
    fn insert(&mut self, asset_id: AssetId<VideoElement>, video: web_sys::HtmlVideoElement) {
        self.elements
            .insert(asset_id, RegisteredElement::new(video));
    }

    fn remove(&mut self, asset_id: impl Into<AssetId<VideoElement>>) -> Option<RegisteredElement> {
        self.elements.remove(&asset_id.into())
    }

    pub fn get(&self, asset_id: impl Into<AssetId<VideoElement>>) -> Option<&RegisteredElement> {
        self.elements.get(&asset_id.into())
    }

    pub fn get_mut(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
    ) -> Option<&mut RegisteredElement> {
        self.elements.get_mut(&asset_id.into())
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

#[derive(Debug)]
pub struct RegisteredElement {
    element: web_sys::HtmlVideoElement,
    listeners: Vec<EventListener>,
}

impl RegisteredElement {
    fn new(element: web_sys::HtmlVideoElement) -> Self {
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
