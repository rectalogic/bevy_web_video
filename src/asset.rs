use crate::{
    event,
    listener::{EventType, ListenerCommand},
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use gloo_events::EventListener;
use std::{cell::RefCell, collections::HashMap, hash::Hash};
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

// wasm on web is single threaded, so this should be OK
thread_local! {
    static REGISTRY: RefCell<Registry> =  RefCell::new(Registry::new());
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegistryId(u64);

impl RegistryId {
    fn new() -> Self {
        Self(0)
    }

    fn increment(&mut self) {
        self.0 += 1;
    }
}

pub struct Registry {
    tx: crossbeam_channel::Sender<ListenerCommand>,
    rx: crossbeam_channel::Receiver<ListenerCommand>,
    next_id: RegistryId,
    elements: HashMap<RegistryId, VideoElement>,
}

impl Registry {
    fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            tx,
            rx,
            next_id: RegistryId::new(),
            elements: HashMap::default(),
        }
    }

    fn allocate_id(&mut self) -> RegistryId {
        let id = self.next_id;
        self.next_id.increment();
        id
    }

    fn add(&mut self, video_element: VideoElement) -> RegistryId {
        let id = self.allocate_id();
        self.insert(id, video_element);
        id
    }

    fn insert(&mut self, registry_id: RegistryId, video_element: VideoElement) {
        self.elements.insert(registry_id, video_element);
    }

    fn remove(&mut self, registry_id: RegistryId) -> Option<VideoElement> {
        self.elements.remove(&registry_id)
    }

    pub fn get(&self, registry_id: &RegistryId) -> Option<&VideoElement> {
        self.elements.get(registry_id)
    }

    pub fn get_mut(&mut self, registry_id: &RegistryId) -> Option<&mut VideoElement> {
        self.elements.get_mut(registry_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VideoElement> {
        self.elements.values()
    }

    pub fn with_borrow<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        REGISTRY.with_borrow(f)
    }

    pub fn with_borrow_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        REGISTRY.with_borrow_mut(f)
    }

    pub fn sender(&self) -> crossbeam_channel::Sender<ListenerCommand> {
        self.tx.clone()
    }

    pub fn receiver(&self) -> crossbeam_channel::Receiver<ListenerCommand> {
        self.rx.clone()
    }
}

pub struct VideoElement {
    target_texture_id: AssetId<Image>,
    element: web_sys::HtmlVideoElement,
    loaded: bool,
    listeners: Vec<EventListener>,
}

impl VideoElement {
    pub fn element(&self) -> web_sys::HtmlVideoElement {
        self.element.clone()
    }

    pub fn target_texture_id(&self) -> AssetId<Image> {
        self.target_texture_id
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn add_event_listener(
        &mut self,
        event_name: &'static str,
        tx: crossbeam_channel::Sender<ListenerCommand>,
        command: ListenerCommand,
    ) {
        let callback = move |_event: &web_sys::Event| {
            if let Err(err) = tx.send(command.clone()) {
                warn!("Failed to register listener: {err:?}");
            };
        };
        let listener = EventListener::new(&self.element, event_name, callback);
        self.listeners.push(listener);
    }
}

fn new_image(size: Extent3d) -> Image {
    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image
}

pub trait AddVideoTextureExt {
    fn add_video_texture(&mut self) -> Handle<Image>;
}

impl AddVideoTextureExt for Assets<Image> {
    fn add_video_texture(&mut self) -> Handle<Image> {
        self.get_handle_provider().reserve_handle().typed::<Image>()
    }
}
