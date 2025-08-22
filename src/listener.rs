use std::marker::PhantomData;

use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use crossbeam_channel::unbounded;
use gloo_events::EventListener;

use crate::{VIDEO_ELEMENTS, VideoId};

#[derive(Event)]
pub struct ListenerEvent<E: EventType> {
    video_id: VideoId,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    fn new(video_id: VideoId) -> Self {
        Self {
            video_id,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn video_id(&self) -> VideoId {
        self.video_id
    }
}

pub trait EventType: Sized + Send + Sync + 'static {
    const EVENT_NAME: &'static str;
}

#[derive(Resource)]
struct EventSender<E: EventType>(crossbeam_channel::Sender<(Option<Entity>, ListenerEvent<E>)>);

#[derive(Resource)]
struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<(Option<Entity>, ListenerEvent<E>)>);

#[derive(Event)]
struct RegisterEventListener<E: EventType> {
    video_id: VideoId,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> RegisterEventListener<E> {
    fn new(video_id: VideoId, target: Option<Entity>) -> Self {
        Self {
            video_id,
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
        // Check if already initialized
        if self
            .world()
            .get_resource::<Events<ListenerEvent<E>>>()
            .is_some()
        {
            return self;
        }
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
        let video_id = registration.video_id;
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.get_mut(&video_id) {
                let tx = sender.0.clone();
                let listener =
                    EventListener::new(&video_element.element, E::EVENT_NAME, move |_event| {
                        if let Err(err) = tx.send((target, ListenerEvent::<E>::new(video_id))) {
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
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl EntityAddEventListenerExt for EntityCommands<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        //XXX all of these should verify video_id is in the registry first, and expect_throw

        let target = self.id();
        self.commands()
            .send_event(RegisterEventListener::<E>::new(video_id, Some(target)));
        self.observe(observer)
    }
}

impl EntityAddEventListenerExt for EntityWorldMut<'_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        self.world_scope(|world| {
            world.send_event(RegisterEventListener::<E>::new(video_id, Some(target)));
        });
        self.observe(observer)
    }
}

pub trait AddEventListenerExt {
    fn add_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl AddEventListenerExt for Commands<'_, '_> {
    fn add_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        self.send_event(RegisterEventListener::<E>::new(video_id, None));
        self.add_observer(observer);
        self
    }
}

impl AddEventListenerExt for App {
    fn add_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        self.world_mut()
            .send_event(RegisterEventListener::<E>::new(video_id, None));
        self.add_observer(observer)
    }
}
