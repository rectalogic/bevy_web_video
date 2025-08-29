use crate::VideoElement;
use bevy::prelude::*;
use gloo_events::EventListener;
use std::collections::HashMap;

pub mod asset;

pub fn plugin(app: &mut App) {
    app.add_plugins(asset::plugin)
        .insert_non_send_resource(VideoElementRegistry::new());
}

pub struct VideoElementRegistry {
    elements: HashMap<AssetId<VideoElement>, RegisteredElement>,
}

impl VideoElementRegistry {
    fn new() -> Self {
        Self {
            elements: HashMap::default(),
        }
    }

    pub fn element(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
    ) -> Option<web_sys::HtmlVideoElement> {
        //XXX return a reference
        self.elements
            .get(&asset_id.into())
            .map(|e| e.element().clone())
    }

    pub(crate) fn add_event_listener(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
        listener: EventListener,
    ) {
        if let Some(registered_element) = self.elements.get_mut(&asset_id.into()) {
            registered_element.listeners.push(listener);
        }
    }

    fn insert(&mut self, asset_id: AssetId<VideoElement>, element: web_sys::HtmlVideoElement) {
        self.elements
            .insert(asset_id, RegisteredElement::new(element));
    }

    fn remove(&mut self, asset_id: impl Into<AssetId<VideoElement>>) -> Option<RegisteredElement> {
        self.elements.remove(&asset_id.into())
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

    fn element(&self) -> &web_sys::HtmlVideoElement {
        &self.element
    }
}
