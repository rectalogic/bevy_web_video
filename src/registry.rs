use crate::{
    VideoElement,
    event::{EventType, ListenerEvent},
};
use bevy::prelude::*;
use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap, marker::PhantomData};

pub mod asset;

pub fn plugin(app: &mut App) {
    app.add_plugins(asset::plugin)
        .insert_non_send_resource(VideoElementRegistry::new());
}

#[derive(Clone)]
pub struct VideoElementRegistry {
    _nonsend: PhantomData<*mut u8>,
}

impl VideoElementRegistry {
    fn new() -> Self {
        Self {
            _nonsend: PhantomData,
        }
    }

    pub fn element(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
    ) -> Option<web_sys::HtmlVideoElement> {
        REGISTRY.with_borrow(|registry| registry.get(&asset_id.into()).map(|e| e.element().clone()))
    }

    pub(crate) fn add_event_listener<E: EventType>(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
        element: &web_sys::HtmlVideoElement,
        event: ListenerEvent<E>,
        tx: crossbeam_channel::Sender<ListenerEvent<E>>,
    ) {
        let listener =
            EventListener::new(element, E::EVENT_NAME, move |_event: &web_sys::Event| {
                if let Err(err) = tx.send(event) {
                    warn!("Failed to ad video event listener: {err:?}");
                };
            });
        REGISTRY.with_borrow_mut(|registry| {
            if let Some(registered_element) = registry.get_mut(&asset_id.into()) {
                registered_element.listeners.push(listener);
            }
        });
    }

    fn insert(&mut self, asset_id: AssetId<VideoElement>, element: web_sys::HtmlVideoElement) {
        REGISTRY
            .with_borrow_mut(|registry| registry.insert(asset_id, RegisteredElement::new(element)));
    }

    fn remove(&mut self, asset_id: impl Into<AssetId<VideoElement>>) -> Option<RegisteredElement> {
        REGISTRY.with_borrow_mut(|registry| registry.remove(&asset_id.into()))
    }
}

thread_local! {
    static REGISTRY: RefCell<HashMap<AssetId<VideoElement>, RegisteredElement>> =  RefCell::new(HashMap::default());
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

    fn element(&self) -> &web_sys::HtmlVideoElement {
        &self.element
    }
}
