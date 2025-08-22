use std::marker::PhantomData;

use bevy::{asset::AsAssetId, ecs::system::IntoObserverSystem, prelude::*};
use crossbeam_channel::unbounded;
use gloo_events::EventListener;

use crate::{VIDEO_ELEMENTS, WebVideo};

#[derive(Event)]
pub struct ListenerEvent<E: EventType> {
    asset_id: AssetId<Image>,
    event_type: E,
}

impl<E: EventType> ListenerEvent<E> {
    pub(crate) fn asset_id(&self) -> AssetId<Image> {
        self.asset_id
    }
}

pub trait EventType: Sized + Send + Sync + 'static {
    const EVENT_NAME: &'static str;

    fn new() -> Self;
}

#[derive(Resource)]
struct EventSender<E: EventType>(crossbeam_channel::Sender<(Option<Entity>, ListenerEvent<E>)>);

#[derive(Resource)]
struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<(Option<Entity>, ListenerEvent<E>)>);

#[derive(Event)]
struct RegisterEventListener<E: EventType> {
    asset_id: AssetId<Image>,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> RegisterEventListener<E> {
    fn new(asset_id: AssetId<Image>, target: Option<Entity>) -> Self {
        Self {
            asset_id,
            target,
            _phantom: PhantomData,
        }
    }
}

pub trait EventListenerApp {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self;
}

impl EventListenerApp for App {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self {
        let (tx, rx) = unbounded();
        self.add_event::<ListenerEvent<E>>()
            .add_event::<RegisterEventListener<E>>()
            .insert_resource(EventSender::<E>(tx))
            .insert_resource(EventReceiver::<E>(rx))
            .add_systems(
                Update,
                (handle_listener_registration::<E>, listen_for_events::<E>),
            )
    }
}

fn handle_listener_registration<E: EventType>(
    mut registrations: EventReader<RegisterEventListener<E>>,
    sender: Res<EventSender<E>>,
) {
    for registration in registrations.read() {
        let target = registration.target;
        let asset_id = registration.asset_id;
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.get_mut(&asset_id) {
                let tx = sender.0.clone();
                let listener =
                    EventListener::new(&video_element.element, E::EVENT_NAME, move |_event| {
                        if let Err(err) = tx.send((
                            target,
                            ListenerEvent {
                                asset_id,
                                event_type: E::new(),
                            },
                        )) {
                            warn!("Failed to register listener: {err:?}");
                        };
                    });
                video_element.listeners.push(listener);
            }
        });
    }
}

fn listen_for_events<E: EventType>(receiver: Res<EventReceiver<E>>, mut commands: Commands) {
    while let Ok((target, event)) = receiver.0.try_recv() {
        if let Some(target) = target {
            commands.trigger_targets(event, target);
        } else {
            commands.trigger(event);
        }
    }
}

pub trait EntityAddEventListenerExt {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl EntityAddEventListenerExt for EntityCommands<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        self.commands().send_event(RegisterEventListener::<E>::new(
            video.as_asset_id(),
            Some(target),
        ));
        self.observe(observer)
    }
}

impl EntityAddEventListenerExt for EntityWorldMut<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        self.world_scope(|world| {
            world.send_event(RegisterEventListener::<E>::new(
                video.as_asset_id(),
                Some(target),
            ));
        });
        self.observe(observer)
    }
}

pub trait AddEventListenerExt {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl AddEventListenerExt for Commands<'_, '_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        self.send_event(RegisterEventListener::<E>::new(video.as_asset_id(), None));
        self.add_observer(observer);
        self
    }
}

impl AddEventListenerExt for App {
    fn add_event_listener<E, B, M>(
        &mut self,
        video: &WebVideo,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        self.world_mut()
            .send_event(RegisterEventListener::<E>::new(video.as_asset_id(), None));
        self.add_observer(observer)
    }
}
