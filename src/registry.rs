use crate::{EventSender, EventType, ListenerEvent, VideoElement, events};
use bevy::prelude::*;
use gloo_events::EventListener;
use std::collections::HashMap;
use wasm_bindgen::UnwrapThrowExt;

pub mod asset;

pub fn plugin(app: &mut App) {
    app.add_plugins(asset::plugin);

    let world = app.world();
    let tx_loadedmetadata = world
        .get_resource::<EventSender<events::LoadedMetadata>>()
        .expect("EventSender<LoadedMetadata>")
        .tx();
    let tx_canplay = world
        .get_resource::<EventSender<events::CanPlay>>()
        .expect("EventSender<CanPlay>")
        .tx();
    let tx_resize = world
        .get_resource::<EventSender<events::Resize>>()
        .expect("EventSender<Resize>")
        .tx();
    let tx_playing = world
        .get_resource::<EventSender<events::Playing>>()
        .expect("EventSender<Playing>")
        .tx();
    let tx_ended = world
        .get_resource::<EventSender<events::Ended>>()
        .expect("EventSender<Ended>")
        .tx();
    let tx_error = world
        .get_resource::<EventSender<events::Error>>()
        .expect("EventSender<Error>")
        .tx();

    app.insert_non_send_resource(VideoElementRegistry::new(
        tx_loadedmetadata,
        tx_canplay,
        tx_resize,
        tx_playing,
        tx_ended,
        tx_error,
    ));
}

pub struct VideoElementRegistry {
    elements: HashMap<AssetId<VideoElement>, RegisteredElement>,
    document: web_sys::Document,
    tx_loadedmetadata: crossbeam_channel::Sender<ListenerEvent<events::LoadedMetadata>>,
    tx_canplay: crossbeam_channel::Sender<ListenerEvent<events::CanPlay>>,
    tx_resize: crossbeam_channel::Sender<ListenerEvent<events::Resize>>,
    tx_playing: crossbeam_channel::Sender<ListenerEvent<events::Playing>>,
    tx_ended: crossbeam_channel::Sender<ListenerEvent<events::Ended>>,
    tx_error: crossbeam_channel::Sender<ListenerEvent<events::Error>>,
}

impl VideoElementRegistry {
    fn new(
        tx_loadedmetadata: crossbeam_channel::Sender<ListenerEvent<events::LoadedMetadata>>,
        tx_canplay: crossbeam_channel::Sender<ListenerEvent<events::CanPlay>>,
        tx_resize: crossbeam_channel::Sender<ListenerEvent<events::Resize>>,
        tx_playing: crossbeam_channel::Sender<ListenerEvent<events::Playing>>,
        tx_ended: crossbeam_channel::Sender<ListenerEvent<events::Ended>>,
        tx_error: crossbeam_channel::Sender<ListenerEvent<events::Error>>,
    ) -> Self {
        Self {
            elements: HashMap::default(),
            document: web_sys::window()
                .expect_throw("window")
                .document()
                .expect_throw("document"),
            tx_loadedmetadata,
            tx_canplay,
            tx_resize,
            tx_playing,
            tx_ended,
            tx_error,
        }
    }

    pub fn element(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
    ) -> Option<&web_sys::HtmlVideoElement> {
        self.elements.get(&asset_id.into()).map(|e| e.element())
    }

    pub fn document(&self) -> &web_sys::Document {
        &self.document
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
        let mut registered_element = RegisteredElement::new(element.clone());

        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_loadedmetadata.clone(),
                asset_id,
                &element,
            ));
        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_canplay.clone(),
                asset_id,
                &element,
            ));
        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_resize.clone(),
                asset_id,
                &element,
            ));
        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_playing.clone(),
                asset_id,
                &element,
            ));
        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_ended.clone(),
                asset_id,
                &element,
            ));
        registered_element
            .listeners
            .push(Self::new_internal_listener(
                self.tx_error.clone(),
                asset_id,
                &element,
            ));

        self.elements.insert(asset_id, registered_element);
    }

    fn new_internal_listener<E: EventType>(
        tx: crossbeam_channel::Sender<ListenerEvent<E>>,
        asset_id: AssetId<VideoElement>,
        element: &web_sys::HtmlVideoElement,
    ) -> EventListener {
        EventListener::new(element, E::EVENT_NAME, move |_event: &web_sys::Event| {
            if let Err(err) = tx.send(ListenerEvent::<E>::new(asset_id, None)) {
                warn!("Failed to fire video event {}: {err:?}", E::EVENT_NAME);
            };
        })
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
