use crate::{
    WebVideo,
    event::{EventSender, EventWithVideoElementId, ListenerEvent, events},
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
        .add_event::<VideoElementCreated>()
        .add_observer(on_loadedmetadata)
        .add_observer(on_resize)
        .add_observer(on_error)
        .add_observer(on_playing)
        .add_systems(PostUpdate, add_listeners.after(AssetEvents));
}

#[derive(Event)]
pub struct VideoElementCreated {
    video_element_id: AssetId<VideoElement>,
}

impl VideoElementCreated {
    fn new(video_element_id: impl Into<AssetId<VideoElement>>) -> Self {
        Self {
            video_element_id: video_element_id.into(),
        }
    }
}

impl EventWithVideoElementId for VideoElementCreated {
    fn video_element_id(&self) -> AssetId<VideoElement> {
        self.video_element_id
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct VideoElement {
    target_texture: Handle<Image>,
    renderable: bool,
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

        let element = RegisteredElement::new(html_video_element);
        let registry_id = ElementRegistry::with_borrow_mut(|registry| registry.add(element));
        Self {
            target_texture,
            renderable: false,
            registry_id,
        }
    }

    pub fn target_texture(&self) -> &Handle<Image> {
        &self.target_texture
    }

    pub fn element(&self) -> Option<web_sys::HtmlVideoElement> {
        ElementRegistry::with_borrow(|registry| {
            registry.get(&self.registry_id).map(|e| e.element().clone())
        })
    }

    pub(crate) fn is_renderable(&self) -> bool {
        self.renderable
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
        if let AssetEvent::Added {
            id: video_element_id,
        } = *event
            && let Some(video_element) = video_elements.get(video_element_id)
        {
            let registry_id = video_element.registry_id();
            ElementRegistry::with_borrow_mut(|registry| {
                if let Some(element) = registry.get_mut(&registry_id) {
                    element.add_event_listener(
                        ListenerEvent::<events::LoadedMetadata>::new(video_element_id, None),
                        loadedmetadata_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Resize>::new(video_element_id, None),
                        resize_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Playing>::new(video_element_id, None),
                        playing_event_sender.tx(),
                    );
                    element.add_event_listener(
                        ListenerEvent::<events::Error>::new(video_element_id, None),
                        error_event_sender.tx(),
                    );
                }
            });

            // Now that we have registered our listeners, allow user to access the element
            web_videos
                .iter()
                .filter_map(|(entity, web_video)| {
                    if web_video.as_asset_id() == video_element_id {
                        Some((entity, video_element_id))
                    } else {
                        None
                    }
                })
                .for_each(|(entity, video_element_id)| {
                    commands.trigger_targets(VideoElementCreated::new(video_element_id), entity)
                });
        }
    }
}

fn resize_image(video_element: &VideoElement, images: &mut Assets<Image>) {
    let Some(element) = video_element.element() else {
        return;
    };
    let mut image = Image::new_uninit(
        Extent3d {
            width: element.video_width(),
            height: element.video_height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    images.insert(video_element.target_texture(), image);
}

fn on_loadedmetadata(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(video_element) = video_elements.get(trigger.video_element_id()) else {
        return;
    };
    resize_image(video_element, &mut images);
}

fn on_resize(
    trigger: Trigger<ListenerEvent<events::Resize>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(video_element) = video_elements.get(trigger.video_element_id()) else {
        return;
    };
    resize_image(video_element, &mut images);
}

fn on_error(
    trigger: Trigger<ListenerEvent<events::Error>>,
    video_elements: Res<Assets<VideoElement>>,
) {
    let Some(video_element) = video_elements.get(trigger.video_element_id()) else {
        return;
    };
    if let Some(element) = video_element.element() {
        warn!("Video {:?} failed with error", element);
    }
}

fn on_playing(
    trigger: Trigger<ListenerEvent<events::Playing>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) {
    let Some(video_element) = video_elements.get_mut(trigger.video_element_id()) else {
        return;
    };
    video_element.renderable = true;
}
