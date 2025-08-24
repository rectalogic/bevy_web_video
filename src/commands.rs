use bevy::prelude::*;

use crate::{WebVideo, asset::VideoSource, registry::Registry};

pub trait EntityCommandsWithVideoElementExt {
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) + Send + Sync + 'static,
    ) -> &mut Self;
}

impl EntityCommandsWithVideoElementExt for EntityCommands<'_> {
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.queue(|mut entity: EntityWorldMut| {
            entity.with_video_element(f);
        })
    }
}

impl EntityCommandsWithVideoElementExt for EntityWorldMut<'_> {
    fn with_video_element(
        &mut self,
        f: impl FnOnce(Option<web_sys::HtmlVideoElement>) + Send + Sync + 'static,
    ) -> &mut Self {
        if let Some(WebVideo(source_handle)) = self.get::<WebVideo>()
            && let Some(sources) = self.get_resource::<Assets<VideoSource>>()
            && let Some(source) = sources.get(source_handle)
        {
            Registry::with_borrow_mut(|registry| {
                if let Some(video_element) = registry.get_mut(&source.registry_id()) {
                    f(Some(video_element.element()));
                } else {
                    f(None);
                }
            });
        } else {
            f(None);
        }
        self
    }
}
