use crate::{
    event::{self, EventSender, ListenerEvent},
    registry::{Registry, RegistryId, VideoElement},
};
use bevy::{asset::AssetEvents, prelude::*};
use wasm_bindgen::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_asset::<VideoSource>()
        //XXX timing issues, need this to run before user configures the video
        .add_systems(PostUpdate, add_listeners.after(AssetEvents));
}

#[derive(Asset, Debug, TypePath)]
pub struct VideoSource {
    target_texture: Handle<Image>,
    registry_id: RegistryId,
}

impl VideoSource {
    pub fn new(target_texture: Handle<Image>) -> Self {
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

        let video_element = VideoElement::new(target_texture.id(), html_video_element);
        let registry_id = Registry::with_borrow_mut(|registry| registry.add(video_element));
        Self {
            target_texture,
            registry_id,
        }
    }

    pub fn target_texture(&self) -> &Handle<Image> {
        &self.target_texture
    }

    pub(crate) fn registry_id(&self) -> RegistryId {
        self.registry_id
    }
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        Registry::with_borrow_mut(|registry| registry.remove(self.registry_id()));
    }
}

fn add_listeners(
    mut events: EventReader<AssetEvent<VideoSource>>,
    sources: Res<Assets<VideoSource>>,
    loadedmetadata_event_sender: Res<EventSender<event::LoadedMetadata>>,
    resize_event_sender: Res<EventSender<event::Resize>>,
    playing_event_sender: Res<EventSender<event::Playing>>,
    error_event_sender: Res<EventSender<event::Error>>,
) {
    for event in events.read() {
        if let AssetEvent::Added { id: asset_id } = event
            && let Some(source) = sources.get(*asset_id)
        {
            Registry::with_borrow_mut(|registry| {
                let registry_id = source.registry_id();
                if let Some(video_element) = registry.get_mut(&registry_id) {
                    video_element.add_event_listener(
                        ListenerEvent::<event::LoadedMetadata>::new(registry_id, None),
                        loadedmetadata_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<event::Resize>::new(registry_id, None),
                        resize_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<event::Playing>::new(registry_id, None),
                        playing_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<event::Error>::new(registry_id, None),
                        error_event_sender.tx(),
                    );
                }
            });
        }
    }
}

pub trait AddVideoTextureExt {
    fn add_video_texture(&mut self) -> Handle<Image>;
}

impl AddVideoTextureExt for Assets<Image> {
    fn add_video_texture(&mut self) -> Handle<Image> {
        self.get_handle_provider().reserve_handle().typed::<Image>()
    }
}
