use crate::{
    WebVideo,
    asset::VideoSource,
    registry::{Registry, RegistryId},
};
use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use std::{marker::PhantomData, sync::Arc};

#[derive(Event)]
pub struct ListenerEvent<E: EventType> {
    registry_id: RegistryId,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    fn new(registry_id: RegistryId, target: Option<Entity>) -> Self {
        Self {
            registry_id,
            target,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn registry_id(&self) -> RegistryId {
        self.registry_id
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

pub trait EntityAddVideoEventListenerExt {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl EntityAddVideoEventListenerExt for EntityCommands<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        self.queue(|mut entity: EntityWorldMut| {
            entity.add_video_event_listener(observer);
        })
    }
}

impl EntityAddVideoEventListenerExt for EntityWorldMut<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        if let Some(WebVideo(source_handle)) = self.get::<WebVideo>()
            && let Some(sources) = self.get_resource::<Assets<VideoSource>>()
            && let Some(source) = sources.get(source_handle)
        {
            Registry::with_borrow_mut(|registry| {
                let entity = self.id();
                let registry_id = source.registry_id();
                let tx = registry.sender();
                if let Some(video_element) = registry.get_mut(&registry_id) {
                    video_element.add_event_listener(
                        E::EVENT_NAME,
                        tx,
                        ListenerCommand::new(move |world| {
                            world.trigger_targets(
                                ListenerEvent::<E>::new(registry_id, Some(entity)),
                                entity,
                            );
                        }),
                    );
                } else {
                    warn!("VideoSource asset {source:?} not found");
                }
            });
        } else {
            warn!(
                "Failed to add video event listener to entity {}, no WebVideo VideoSource found",
                self.id()
            );
        }

        self.observe(observer)
    }
}
