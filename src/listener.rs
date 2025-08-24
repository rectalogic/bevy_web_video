use crate::{
    WebVideo,
    asset::VideoSource,
    event::{EventType, ListenerEvent},
    registry::Registry,
};
use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use std::sync::Arc;

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
