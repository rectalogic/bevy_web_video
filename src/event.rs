use crate::{VideoElement, VideoElementRegistry};
use bevy::prelude::*;
use crossbeam_channel::unbounded;
use gloo_events::EventListener;
use std::marker::PhantomData;

pub fn plugin(app: &mut App) {
    app.add_listener_event::<events::LoadedMetadata>()
        .add_listener_event::<events::CanPlay>()
        .add_listener_event::<events::Resize>()
        .add_listener_event::<events::Playing>()
        .add_listener_event::<events::Ended>()
        .add_listener_event::<events::Error>();
}

pub trait EventListenerAppExt {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self;
}

impl EventListenerAppExt for App {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self {
        // Check if already initialized
        if self.world().contains_resource::<Events<ListenerEvent<E>>>() {
            return self;
        }
        let (tx, rx) = unbounded();
        self.add_event::<ListenerEvent<E>>()
            .insert_resource(EventSender::<E>(tx))
            .insert_resource(EventReceiver::<E>(rx))
            .add_systems(Update, listen_for_events::<E>)
    }
}

#[derive(Resource)]
pub struct EventSender<E: EventType>(crossbeam_channel::Sender<ListenerEvent<E>>);

impl<E: EventType> EventSender<E> {
    pub fn enable_element_event_observers(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
        element: &web_sys::EventTarget,
        registry: &mut VideoElementRegistry,
        target: Entity,
    ) -> &Self {
        let tx = self.0.clone();
        let asset_id = asset_id.into();
        let listener =
            EventListener::new(element, E::EVENT_NAME, move |_event: &web_sys::Event| {
                if let Err(err) = tx.send(ListenerEvent::<E>::new(asset_id, Some(target))) {
                    warn!("Failed to fire video event {}: {err:?}", E::EVENT_NAME);
                };
            });
        registry.add_event_listener(asset_id, listener);
        self
    }

    pub(crate) fn tx(&self) -> crossbeam_channel::Sender<ListenerEvent<E>> {
        self.0.clone()
    }
}

#[derive(Resource)]
struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<ListenerEvent<E>>);

#[derive(Event, Clone, Debug)]
pub struct ListenerEvent<E: EventType> {
    asset_id: AssetId<VideoElement>,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    pub(crate) fn new(asset_id: AssetId<VideoElement>, target: Option<Entity>) -> Self {
        Self {
            asset_id,
            target,
            _phantom: PhantomData,
        }
    }

    pub fn asset_id(&self) -> AssetId<VideoElement> {
        self.asset_id
    }
}

pub trait EventType: Copy + Clone + Send + Sync + 'static {
    const EVENT_NAME: &'static str;
}

pub mod events {
    use super::*;

    #[macro_export]
    macro_rules! new_event_type {
        ($name:ident, $event_name:literal) => {
            #[derive(Event, Copy, Clone, Debug)]
            pub struct $name;

            impl $crate::EventType for $name {
                const EVENT_NAME: &'static str = $event_name;
            }
        };
    }

    new_event_type!(LoadedMetadata, "loadedmetadata");
    new_event_type!(CanPlay, "canplay");
    new_event_type!(Resize, "resize");
    new_event_type!(Playing, "playing");
    new_event_type!(Ended, "ended");
    new_event_type!(Error, "error");
}

fn listen_for_events<E: EventType>(receiver: Res<EventReceiver<E>>, mut commands: Commands) {
    while let Ok(event) = receiver.0.try_recv() {
        if let Some(target) = event.target {
            commands.trigger_targets(event, target);
        } else {
            commands.trigger(event);
        }
    }
}
