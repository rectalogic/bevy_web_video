use crate::{
    VideoElementRegistry, WebVideo,
    event::{EventSender, EventWithAssetId, ListenerEvent, events},
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
        .add_systems(Update, mark_assets_modified)
        .add_systems(PostUpdate, asset_events.after(AssetEvents));
}

#[derive(Event)]
pub struct VideoElementCreated {
    asset_id: AssetId<VideoElement>,
}

impl VideoElementCreated {
    fn new(asset_id: impl Into<AssetId<VideoElement>>) -> Self {
        Self {
            asset_id: asset_id.into(),
        }
    }
}

impl EventWithAssetId for VideoElementCreated {
    type Asset = VideoElement;
    fn asset_id(&self) -> AssetId<VideoElement> {
        self.asset_id
    }
}

#[derive(Asset, Clone, Debug, TypePath)]
pub struct VideoElement {
    target_image_id: AssetId<Image>,
    renderable: bool,
    asset_id: Option<AssetId<VideoElement>>,
}

impl VideoElement {
    pub fn new(target_image: impl Into<AssetId<Image>>) -> Self {
        Self {
            target_image_id: target_image.into(),
            renderable: false,
            asset_id: None,
        }
    }

    pub fn target_image_id(&self) -> AssetId<Image> {
        self.target_image_id
    }

    pub(crate) fn is_renderable(&self) -> bool {
        self.renderable
    }
}

fn mark_assets_modified(mut video_elements: ResMut<Assets<VideoElement>>) {
    // Mark modified every frame so RenderAsset prepares the texture
    video_elements.iter_mut().for_each(drop);
}

#[allow(clippy::too_many_arguments)]
fn asset_events(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<VideoElement>>,
    web_videos: Query<(Entity, &WebVideo)>,
    mut video_elements: ResMut<Assets<VideoElement>>,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    resize_event_sender: Res<EventSender<events::Resize>>,
    playing_event_sender: Res<EventSender<events::Playing>>,
    error_event_sender: Res<EventSender<events::Error>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) {
    for event in events.read() {
        match *event {
            AssetEvent::Removed { id: asset_id } | AssetEvent::Unused { id: asset_id } => {
                registry.remove(asset_id);
            }
            AssetEvent::Added { id: asset_id } => {
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

                // Insert before adding listeners
                registry.insert(asset_id, html_video_element.clone());

                loadedmetadata_event_sender.add_video_event_listener_internal(
                    asset_id,
                    html_video_element.as_ref(),
                    &mut registry,
                );
                resize_event_sender.add_video_event_listener_internal(
                    asset_id,
                    html_video_element.as_ref(),
                    &mut registry,
                );
                playing_event_sender.add_video_event_listener_internal(
                    asset_id,
                    html_video_element.as_ref(),
                    &mut registry,
                );
                error_event_sender.add_video_event_listener_internal(
                    asset_id,
                    html_video_element.as_ref(),
                    &mut registry,
                );

                let video_element = video_elements.get_mut(asset_id).expect("VideoElement");
                video_element.asset_id = Some(asset_id);

                // Now that we have registered our listeners, allow user to access the element
                web_videos
                    .iter()
                    .filter_map(|(entity, web_video)| {
                        if web_video.as_asset_id() == asset_id {
                            Some((entity, asset_id))
                        } else {
                            None
                        }
                    })
                    .for_each(|(entity, asset_id)| {
                        commands.trigger_targets(VideoElementCreated::new(asset_id), entity)
                    });
            }
            _ => {}
        }
    }
}

fn resize_image(
    video_element: &VideoElement,
    element: &web_sys::HtmlVideoElement,
    images: &mut Assets<Image>,
) {
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
    images.insert(video_element.target_image_id(), image);
}

fn on_loadedmetadata(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
    registry: NonSend<VideoElementRegistry>,
) {
    let asset_id = trigger.asset_id();
    if let Some(video_element) = video_elements.get(asset_id)
        && let Some(element) = registry.element(asset_id)
    {
        resize_image(video_element, &element, &mut images);
    };
}

fn on_resize(
    trigger: Trigger<ListenerEvent<events::Resize>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
    registry: NonSend<VideoElementRegistry>,
) {
    let asset_id = trigger.asset_id();
    if let Some(video_element) = video_elements.get(asset_id)
        && let Some(element) = registry.element(asset_id)
    {
        resize_image(video_element, &element, &mut images);
    };
}

fn on_error(
    trigger: Trigger<ListenerEvent<events::Error>>,
    video_elements: Res<Assets<VideoElement>>,
) {
    let asset_id = trigger.asset_id();
    if video_elements.get(asset_id).is_none() {
        warn!("Video asset {:?} failed to load with error", asset_id);
    };
}

fn on_playing(
    trigger: Trigger<ListenerEvent<events::Playing>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) {
    let Some(video_element) = video_elements.get_mut(trigger.asset_id()) else {
        return;
    };
    video_element.renderable = true;
}
