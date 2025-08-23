use std::marker::PhantomData;

use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use crossbeam_channel::unbounded;
use gloo_events::EventListener;

use crate::{VIDEO_ELEMENTS, VideoId};

#[derive(Event)]
pub struct ListenerEvent<E: EventType> {
    video_id: VideoId,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    fn new(video_id: VideoId, target: Option<Entity>) -> Self {
        Self {
            video_id,
            target,
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
struct EventSender<E: EventType>(crossbeam_channel::Sender<ListenerEvent<E>>);

#[derive(Resource)]
struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<ListenerEvent<E>>);

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
            .insert_resource(EventSender::<E>(tx))
            .insert_resource(EventReceiver::<E>(rx))
            .add_systems(Update, listen_for_events::<E>)
    }
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

fn register_event_listener<E: EventType>(
    video_id: VideoId,
    target: Option<Entity>,
    sender: crossbeam_channel::Sender<ListenerEvent<E>>,
) {
    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
        if let Some(video_element) = elements.get_mut(&video_id) {
            let listener =
                EventListener::new(&video_element.element, E::EVENT_NAME, move |_event| {
                    if let Err(err) = sender.send(ListenerEvent::<E>::new(video_id, target)) {
                        warn!("Failed to register listener: {err:?}");
                    };
                });
            video_element.listeners.push(listener);
        }
    });
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
        let target = self.id();
        self.commands().queue(move |world: &mut World| {
            if let Some(sender) = world.get_resource::<EventSender<E>>() {
                register_event_listener(video_id, Some(target), sender.0.clone());
            }
        });
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
            if let Some(sender) = world.get_resource::<EventSender<E>>() {
                register_event_listener(video_id, Some(target), sender.0.clone());
            }
        });
        self.observe(observer)
    }
}

pub(crate) trait AddEventListenerExt {
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
        self.queue(move |world: &mut World| {
            if let Some(sender) = world.get_resource::<EventSender<E>>() {
                register_event_listener(video_id, None, sender.0.clone());
            }
        });
        self.add_observer(observer);
        self
    }
}
