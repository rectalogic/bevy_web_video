use std::{marker::PhantomData, rc::Rc, sync::Arc};

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

pub trait ListenerCallback: FnMut(&mut World) + Send + Sync + 'static {}
impl<C: FnMut(&mut World) + Send + Sync + 'static> ListenerCallback for C {}

#[derive(Clone)]
pub struct ListenerCommand(Arc<Box<dyn ListenerCallback>>);

impl ListenerCommand {
    pub fn new(command: impl ListenerCallback) -> Self {
        Self(Arc::new(Box::new(command)))
    }
}

impl Command for ListenerCommand {
    fn apply(mut self, world: &mut World) {
        Arc::get_mut(&mut self.0).unwrap()(world);
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
        self.add_event::<ListenerEvent<E>>()
    }
}

pub trait EntityAddVideoEventListenerExt {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl EntityAddVideoEventListenerExt for EntityCommands<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.elements.get_mut(&video_id) {
                let tx = elements.tx.clone();
                video_element.add_event_listener(
                    E::EVENT_NAME,
                    tx,
                    ListenerCommand::new(move |world| {
                        world.trigger_targets(
                            ListenerEvent::<E>::new(video_id, Some(target)),
                            target,
                        );
                    }),
                );
            }
        });
        self.observe(observer)
    }
}

impl EntityAddVideoEventListenerExt for EntityWorldMut<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.elements.get_mut(&video_id) {
                let tx = elements.tx.clone();
                video_element.add_event_listener(
                    E::EVENT_NAME,
                    tx,
                    ListenerCommand::new(move |world| {
                        world.trigger_targets(
                            ListenerEvent::<E>::new(video_id, Some(target)),
                            target,
                        );
                    }),
                );
            }
        });
        self.observe(observer)
    }
}

pub(crate) trait CommandsAddEventListenerExt {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl CommandsAddEventListenerExt for Commands<'_, '_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        video_id: VideoId,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            if let Some(video_element) = elements.elements.get_mut(&video_id) {
                let tx = elements.tx.clone();
                video_element.add_event_listener(
                    E::EVENT_NAME,
                    tx,
                    ListenerCommand::new(move |world| {
                        world.trigger(ListenerEvent::<E>::new(video_id, None));
                    }),
                );
            }
        });
        self.add_observer(observer);
        self
    }
}
