use crate::{
    WebVideo,
    event::{EventSender, ListenerEvent, events},
    registry::{ElementRegistry, RegisteredElement, RegistryId},
};
use bevy::{
    asset::{AsAssetId, AssetEvents, RenderAssetUsages},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use wasm_bindgen::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_asset::<VideoElement>()
        .add_event::<VideoCreated>()
        .add_observer(on_loadedmetadata)
        .add_observer(on_resize)
        .add_observer(on_error)
        .add_observer(on_playing)
        .add_systems(PostUpdate, add_listeners.after(AssetEvents));
}

#[derive(Event)]
pub struct VideoCreated {
    registry_id: RegistryId,
    asset_id: AssetId<VideoElement>,
}

impl VideoCreated {
    fn new(registry_id: RegistryId, asset_id: AssetId<VideoElement>) -> Self {
        Self {
            registry_id,
            asset_id,
        }
    }

    pub fn asset_id(&self) -> AssetId<VideoElement> {
        self.asset_id
    }

    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        ElementRegistry::with_borrow(|registry| {
            registry.get(&self.registry_id).map(|e| e.element())
        })
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct VideoElement {
    target_texture: Handle<Image>,
    registry_id: RegistryId,
}

impl VideoElement {
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

        let element = RegisteredElement::new(target_texture.id(), html_video_element);
        let registry_id = ElementRegistry::with_borrow_mut(|registry| registry.add(element));
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

impl Drop for VideoElement {
    fn drop(&mut self) {
        ElementRegistry::with_borrow_mut(|registry| registry.remove(self.registry_id()));
    }
}

#[allow(clippy::too_many_arguments)]
fn add_listeners(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<VideoElement>>,
    web_videos: Query<(Entity, &WebVideo)>,
    video_elements: Res<Assets<VideoElement>>,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    resize_event_sender: Res<EventSender<events::Resize>>,
    playing_event_sender: Res<EventSender<events::Playing>>,
    error_event_sender: Res<EventSender<events::Error>>,
) {
    for event in events.read() {
        if let AssetEvent::Added { id: asset_id } = *event
            && let Some(source) = video_elements.get(asset_id)
        {
            let registry_id = source.registry_id();
            ElementRegistry::with_borrow_mut(|registry| {
                if let Some(element) = registry.get_mut(&registry_id) {
                    element.add_event_listener(
                        ListenerEvent::<events::LoadedMetadata>::new(registry_id, None),
                        loadedmetadata_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Resize>::new(registry_id, None),
                        resize_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Playing>::new(registry_id, None),
                        playing_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Error>::new(registry_id, None),
                        error_event_sender.tx(),
                    );
                }
            });

            // Now that we have registered our listeners, allow user to access the element
            web_videos
                .iter()
                .filter_map(|(entity, web_video)| {
                    if web_video.as_asset_id() == asset_id {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .for_each(|entity| {
                    commands.trigger_targets(VideoCreated::new(registry_id, asset_id), entity)
                });
        }
    }
}

fn resize_image(element: &RegisteredElement, images: &mut Assets<Image>) {
    let mut image = Image::new_uninit(
        Extent3d {
            width: element.element().video_width(),
            height: element.element().video_height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    images.insert(element.target_texture_id(), image);
}

fn on_loadedmetadata(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mut images: ResMut<Assets<Image>>,
) {
    ElementRegistry::with_borrow(|registry| {
        if let Some(element) = registry.get(&trigger.registry_id()) {
            resize_image(element, &mut images);
        }
    });
}

fn on_resize(trigger: Trigger<ListenerEvent<events::Resize>>, mut images: ResMut<Assets<Image>>) {
    ElementRegistry::with_borrow(|registry| {
        if let Some(element) = registry.get(&trigger.registry_id()) {
            resize_image(element, &mut images);
        }
    });
}

fn on_error(trigger: Trigger<ListenerEvent<events::Error>>) {
    let video = trigger.video_element();
    warn!("Video {video:?} failed with error");
}

fn on_playing(trigger: Trigger<ListenerEvent<events::Playing>>) {
    ElementRegistry::with_borrow_mut(|registry| {
        if let Some(element) = registry.get_mut(&trigger.registry_id()) {
            element.set_renderable();
        }
    });
}
