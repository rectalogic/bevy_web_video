use crate::{
    VideoElement, VideoElementRegistry,
    event::{EventSender, EventType, ListenerEvent},
};
use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub trait EntityAddVideoEventListenerExt {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle;
}

impl EntityAddVideoEventListenerExt for EntityCommands<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let asset_id = asset_id.into();
        self.queue(move |mut entity: EntityWorldMut| {
            entity.add_video_event_listener(asset_id, observer);
        })
    }
}

//XXX not safe
impl EntityAddVideoEventListenerExt for EntityWorldMut<'_> {
    fn add_video_event_listener<E, B, M>(
        &mut self,
        asset_id: impl Into<AssetId<VideoElement>>,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &mut Self
    where
        E: EventType,
        B: Bundle,
    {
        let target = self.id();
        let asset_id = asset_id.into();

        let Some(event_sender) = self.get_resource::<EventSender<E>>() else {
            warn!("Video event type {} not registered", E::EVENT_NAME);
            return self;
        };
        let tx = event_sender.tx();
        self.world_scope(|world: &mut World| {
            if let Some(mut registry) = world.get_non_send_resource_mut::<VideoElementRegistry>()
                && let Some(element) = registry.element(asset_id)
            {
                registry.add_event_listener(
                    asset_id,
                    &element,
                    ListenerEvent::<E>::new(asset_id, Some(target)),
                    tx,
                );
            }
        });
        self.observe(observer)
    }
}
