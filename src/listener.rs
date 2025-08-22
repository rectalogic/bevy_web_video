use std::marker::PhantomData;

use bevy::{
    asset::AsAssetId,
    ecs::system::{IntoObserverSystem, ObserverSystem},
    prelude::*,
};
use crossbeam_channel::unbounded;
use gloo_events::EventListener;

use crate::{VIDEO_ELEMENTS, WebVideoSink};

pub trait ListenerEvent: Event {
    // XXX need str name and AssetId?
    const EVENT_NAME: &'static str;

    fn new() -> Self;
}

#[derive(Resource)]
struct EventSender<E: ListenerEvent>(crossbeam_channel::Sender<(Entity, E)>);

#[derive(Resource)]
struct EventReceiver<E: ListenerEvent>(crossbeam_channel::Receiver<(Entity, E)>);

#[derive(Event)]
struct RegisterEventListener<E: ListenerEvent> {
    asset_id: AssetId<Image>,
    target: Entity,
    _phantom: PhantomData<E>,
}

impl<E: ListenerEvent> RegisterEventListener<E> {
    fn new(asset_id: AssetId<Image>, target: Entity) -> Self {
        Self {
            asset_id,
            target,
            _phantom: PhantomData,
        }
    }
}

pub trait EventListenerApp {
    fn add_listener_event<E: ListenerEvent>(&mut self) -> &mut Self;
}

impl EventListenerApp for App {
    fn add_listener_event<E: ListenerEvent>(&mut self) -> &mut Self {
        let (tx, rx) = unbounded();
        self.add_event::<E>()
            .add_event::<RegisterEventListener<E>>()
            .insert_resource(EventSender::<E>(tx))
            .insert_resource(EventReceiver::<E>(rx))
            .add_systems(
                Update,
                (handle_listener_registration::<E>, listen_for_events::<E>),
            )
    }
}

fn handle_listener_registration<E: ListenerEvent>(
    mut registrations: EventReader<RegisterEventListener<E>>,
    sender: Res<EventSender<E>>,
) {
    for registration in registrations.read() {
        let target = registration.target;
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.get_mut(&registration.asset_id) {
                let tx = sender.0.clone();
                let listener =
                    EventListener::new(&video_element.element, E::EVENT_NAME, move |_event| {
                        tx.send((target, E::new()));
                    });
                //XXX store listener in video_element HashMap
            }
        });
    }
}

fn listen_for_events<E: ListenerEvent>(receiver: Res<EventReceiver<E>>, mut commands: Commands) {
    while let Ok((target, event)) = receiver.0.try_recv() {
        commands.trigger_targets(event, target);
    }
}

pub trait EntityAddEventListenerExt {
    fn add_event_listener<E, B, M>(
        &mut self,
        sink: &WebVideoSink,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self
    where
        E: ListenerEvent,
        B: Bundle;
}

impl EntityAddEventListenerExt for EntityCommands<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        sink: &WebVideoSink,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self
    where
        E: ListenerEvent,
        B: Bundle,
    {
        let target = self.id();
        self.commands()
            .send_event(RegisterEventListener::<E>::new(sink.as_asset_id(), target));
        self.observe(observer)
    }
}

impl EntityAddEventListenerExt for EntityWorldMut<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        sink: &WebVideoSink,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self
    where
        E: ListenerEvent,
        B: Bundle,
    {
        let target = self.id();
        self.world_scope(|world| {
            world.send_event(RegisterEventListener::<E>::new(sink.as_asset_id(), target));
        });
        self.observe(observer)
    }
}
