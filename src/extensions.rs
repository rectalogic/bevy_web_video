use crate::{
    WebVideo,
    asset::VideoElement,
    event::{EventSender, EventType, ListenerEvent},
    registry::ElementRegistry,
};
use bevy::{asset::AsAssetId, ecs::system::IntoObserverSystem, prelude::*};

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
        let target = self.id();
        let Some(event_sender) = self.get_resource::<EventSender<E>>() else {
            warn!("Video event type {} not registered", E::EVENT_NAME);
            return self;
        };
        let Some(video_elements) = self.get_resource::<Assets<VideoElement>>() else {
            return self;
        };
        let Some(web_video) = self.get::<WebVideo>() else {
            warn!("No WebVideo component found on entity {}", target);
            return self;
        };
        let video_element_id = web_video.as_asset_id();
        let Some(video_element) = video_elements.get(video_element_id) else {
            return self;
        };
        let tx = event_sender.tx();
        let registry_id = video_element.registry_id();
        ElementRegistry::with_borrow_mut(|registry| {
            if let Some(element) = registry.get_mut(&registry_id) {
                element.add_event_listener(
                    ListenerEvent::<E>::new(video_element_id, Some(target)),
                    tx,
                );
                self.observe(observer);
            }
        });
        self
    }
}
