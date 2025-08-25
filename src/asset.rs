use crate::{
    event::{EventSender, ListenerEvent, events},
    registry::{Registry, RegistryId, VideoElement},
};
use bevy::{
    asset::AssetEvents,
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use wasm_bindgen::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_asset::<VideoSource>()
        .add_observer(on_loadedmetadata)
        .add_observer(on_resize)
        .add_observer(on_error)
        .add_observer(on_playing)
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
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    resize_event_sender: Res<EventSender<events::Resize>>,
    playing_event_sender: Res<EventSender<events::Playing>>,
    error_event_sender: Res<EventSender<events::Error>>,
) {
    for event in events.read() {
        if let AssetEvent::Added { id: asset_id } = event
            && let Some(source) = sources.get(*asset_id)
        {
            Registry::with_borrow_mut(|registry| {
                let registry_id = source.registry_id();
                if let Some(video_element) = registry.get_mut(&registry_id) {
                    video_element.add_event_listener(
                        ListenerEvent::<events::LoadedMetadata>::new(registry_id, None),
                        loadedmetadata_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<events::Resize>::new(registry_id, None),
                        resize_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<events::Playing>::new(registry_id, None),
                        playing_event_sender.tx(),
                    );
                    video_element.add_event_listener(
                        ListenerEvent::<events::Error>::new(registry_id, None),
                        error_event_sender.tx(),
                    );
                }
            });
        }
    }
}

fn resize_image(video_element: &VideoElement, images: &mut Assets<Image>) {
    let mut image = Image::new_uninit(
        Extent3d {
            width: video_element.element().video_width(),
            height: video_element.element().video_height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    images.insert(video_element.target_texture_id(), image);
}

fn on_loadedmetadata(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mut images: ResMut<Assets<Image>>,
) {
    Registry::with_borrow(|registry| {
        if let Some(video_element) = registry.get(&trigger.registry_id()) {
            resize_image(video_element, &mut images);
        }
    });
}

fn on_resize(trigger: Trigger<ListenerEvent<events::Resize>>, mut images: ResMut<Assets<Image>>) {
    Registry::with_borrow(|registry| {
        if let Some(video_element) = registry.get(&trigger.registry_id()) {
            resize_image(video_element, &mut images);
        }
    });
}

fn on_error(trigger: Trigger<ListenerEvent<events::Error>>) {
    let video = trigger.video_element();
    warn!("Video {video:?} failed with error");
}

fn on_playing(trigger: Trigger<ListenerEvent<events::Playing>>) {
    Registry::with_borrow_mut(|registry| {
        if let Some(video_element) = registry.get_mut(&trigger.registry_id()) {
            video_element.set_loaded();
        }
    });
}

pub trait AddVideoTextureExt {
    fn add_video_texture(&mut self) -> Handle<Image>;
}

impl AddVideoTextureExt for Assets<Image> {
    fn add_video_texture(&mut self) -> Handle<Image> {
        self.get_handle_provider().reserve_handle().typed::<Image>()
    }
}
