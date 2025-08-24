use crate::{
    event::{self, EventType},
    listener::ListenerCommand,
    registry::{Registry, RegistryId, VideoElement},
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use wasm_bindgen::prelude::*;

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

        let registry_index = Registry::with_borrow_mut(|registry| {
            let width = html_video_element.video_width();
            let height = html_video_element.video_height();

            let mut video_element = VideoElement {
                target_texture_id: target_texture.id(),
                element: html_video_element,
                loaded: false,
                listeners: Vec::new(),
            };

            let resize_callback = {
                let target_texture = target_texture.clone();
                ListenerCommand::new(move |world| {
                    if let Some(mut images) = world.get_resource_mut::<Assets<Image>>() {
                        images.insert(
                            &target_texture,
                            new_image(Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            }),
                        );
                    }
                })
            };

            let tx = registry.tx.clone();
            video_element.add_event_listener(
                event::LoadedMetadata::EVENT_NAME,
                tx,
                resize_callback.clone(),
            );

            let tx = registry.tx.clone();
            video_element.add_event_listener(event::Resize::EVENT_NAME, tx, resize_callback);

            let registry_id = registry.allocate_id();

            let tx = registry.tx.clone();
            video_element.add_event_listener(
                event::Playing::EVENT_NAME,
                tx,
                ListenerCommand::new(move |_world| {
                    Registry::with_borrow_mut(|registry| {
                        if let Some(video_element) = registry.elements.get_mut(&registry_id) {
                            video_element.loaded = true;
                        }
                    });
                }),
            );

            registry.insert(registry_id, video_element);
            registry_id
        });

        Self {
            target_texture,
            registry_id: registry_index,
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

pub trait AddVideoTextureExt {
    fn add_video_texture(&mut self) -> Handle<Image>;
}

impl AddVideoTextureExt for Assets<Image> {
    fn add_video_texture(&mut self) -> Handle<Image> {
        self.get_handle_provider().reserve_handle().typed::<Image>()
    }
}

pub(crate) fn new_image(size: Extent3d) -> Image {
    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image
}
