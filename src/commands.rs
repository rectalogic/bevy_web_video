use bevy::{prelude::*, render::render_resource::Extent3d};
use wasm_bindgen::{JsCast, UnwrapThrowExt};

use crate::{
    VIDEO_ELEMENTS, VideoElement, VideoId, WebVideo,
    event::{LoadedMetadata, Playing, Resize},
    listener::{EventType, ListenerCommand},
    new_image,
};

pub trait CommandsSpawnVideoExt {
    fn spawn_video(&mut self, image: impl Into<AssetId<Image>>) -> EntityCommands<'_>;
}

impl CommandsSpawnVideoExt for Commands<'_, '_> {
    fn spawn_video(&mut self, image: impl Into<AssetId<Image>>) -> EntityCommands<'_> {
        let video_id = VideoId(image.into());
        let html_video_element = web_sys::window()
            .expect_throw("window")
            .document()
            .expect_throw("document")
            .create_element("video")
            .inspect_err(|e| warn!("{e:?}"))
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlVideoElement>()
            .inspect_err(|e| warn!("{e:?}"))
            .expect_throw("web_sys::HtmlVideoElement");

        VIDEO_ELEMENTS.with_borrow_mut(|elements| {
            let mut video_element = VideoElement {
                element: html_video_element,
                loaded: false,
                text_track: None,
                listeners: Vec::new(),
            };

            let width = video_element.element.video_width();
            let height = video_element.element.video_height();
            let resize_callback = ListenerCommand::new(move |world| {
                if let Some(mut images) = world.get_resource_mut::<Assets<Image>>() {
                    images.insert(
                        video_id.0,
                        new_image(Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        }),
                    );
                }
            });

            let tx = elements.tx.clone();
            video_element.add_event_listener(
                LoadedMetadata::EVENT_NAME,
                tx,
                resize_callback.clone(),
            );

            let tx = elements.tx.clone();
            video_element.add_event_listener(
                Playing::EVENT_NAME,
                tx,
                ListenerCommand::new(move |_world| {
                    VIDEO_ELEMENTS.with_borrow_mut(|elements| {
                        if let Some(video_element) = elements.elements.get_mut(&video_id) {
                            video_element.loaded = true;
                        }
                    });
                }),
            );

            let tx = elements.tx.clone();
            video_element.add_event_listener(Resize::EVENT_NAME, tx, resize_callback);

            elements.elements.insert(video_id, video_element)
        });

        self.spawn(WebVideo::new(video_id))
    }
}

pub trait EntityCommandsConfigureVideoExt {
    fn configure_video(
        &mut self,
        configure: impl FnOnce(Option<web_sys::HtmlVideoElement>) + Send + Sync + 'static,
    ) -> &mut Self;
}

impl EntityCommandsConfigureVideoExt for EntityCommands<'_> {
    fn configure_video(
        &mut self,
        configure: impl FnOnce(Option<web_sys::HtmlVideoElement>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.queue(|entity: EntityWorldMut| {
            if let Some(WebVideo(video_id)) = entity.get::<WebVideo>() {
                VIDEO_ELEMENTS.with_borrow_mut(|elements| {
                    if let Some(video_element) = elements.elements.get_mut(&video_id) {
                        configure(Some(video_element.element.clone()));
                    } else {
                        configure(None);
                    }
                });
            } else {
                configure(None);
            }
        });

        self
    }
}
