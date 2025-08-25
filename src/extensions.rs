use crate::{
    WebVideo,
    asset::VideoSource,
    event::{EventSender, EventType, ListenerEvent},
    registry::Registry,
};
use bevy::{ecs::system::IntoObserverSystem, prelude::*};

pub trait AddVideoTextureExt {
    fn add_video_texture(&mut self) -> Handle<Image>;
}

pub trait EntityCommandsWithVideoElementExt {
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) -> Result<()> + Send + Sync + 'static,
    ) -> &mut Self;
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

impl AddVideoTextureExt for Assets<Image> {
    fn add_video_texture(&mut self) -> Handle<Image> {
        self.get_handle_provider().reserve_handle().typed::<Image>()
    }
}

impl EntityCommandsWithVideoElementExt for EntityCommands<'_> {
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) -> Result<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.queue(|mut entity: EntityWorldMut| {
            entity.with_video_element(f);
        })
    }
}

impl EntityCommandsWithVideoElementExt for EntityWorldMut<'_> {
    //XXX this needs to happen after we add internal listeners - use an observer and trigger an event?
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) -> Result<()> + Send + Sync + 'static,
    ) -> &mut Self {
        if let Err(err) = if let Some(WebVideo(source_handle)) = self.get::<WebVideo>()
            && let Some(sources) = self.get_resource::<Assets<VideoSource>>()
            && let Some(source) = sources.get(source_handle)
        {
            Registry::with_borrow_mut(|registry| {
                if let Some(video_element) = registry.get_mut(&source.registry_id()) {
                    f(Some(video_element.element()))
                } else {
                    f(None)
                }
            })
        } else {
            f(None)
        } {
            warn!("with_video_element failed: {err:?}");
        }
        self
    }
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
        let Some(sources) = self.get_resource::<Assets<VideoSource>>() else {
            return self;
        };
        let Some(WebVideo(source_handle)) = self.get::<WebVideo>() else {
            warn!("No WebVideo component found on entity {}", target);
            return self;
        };
        let Some(source) = sources.get(source_handle) else {
            return self;
        };
        let tx = event_sender.tx();
        let registry_id = source.registry_id();

        Registry::with_borrow_mut(|registry| {
            if let Some(video_element) = registry.get_mut(&registry_id) {
                video_element
                    .add_event_listener(ListenerEvent::<E>::new(registry_id, Some(target)), tx);
                self.observe(observer);
            }
        });
        self
    }
}
