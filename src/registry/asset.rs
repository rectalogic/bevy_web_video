use crate::{
    VideoElementRegistry,
    event::{ListenerEvent, events},
};
use bevy::{
    asset::{AssetEvents, RenderAssetUsages},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use wasm_bindgen::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_asset::<VideoElement>()
        .add_observer(on_loadedmetadata)
        .add_observer(on_canplay)
        .add_observer(on_resize)
        .add_observer(on_error)
        .add_observer(on_playing)
        .add_observer(on_ended)
        .add_systems(Update, mark_assets_modified)
        .add_systems(PostUpdate, remove_unused_assets.after(AssetEvents));
}

#[derive(Asset, Clone, Debug, TypePath)]
pub struct VideoElement {
    target_image_id: AssetId<Image>,
    renderable: bool,
}

impl VideoElement {
    fn new(target_image: impl Into<AssetId<Image>>) -> Self {
        Self {
            target_image_id: target_image.into(),
            renderable: false,
        }
    }

    pub fn target_image_id(&self) -> AssetId<Image> {
        self.target_image_id
    }

    pub(crate) fn is_renderable(&self) -> bool {
        self.renderable
    }
}

pub trait VideoElementAssetsExt {
    fn new_video(
        &mut self,
        target_image: impl Into<AssetId<Image>>,
        registry: &mut VideoElementRegistry,
    ) -> (Handle<VideoElement>, web_sys::HtmlVideoElement);
}

impl VideoElementAssetsExt for Assets<VideoElement> {
    fn new_video(
        &mut self,
        target_image: impl Into<AssetId<Image>>,
        registry: &mut VideoElementRegistry,
    ) -> (Handle<VideoElement>, web_sys::HtmlVideoElement) {
        let video_handle = self.reserve_handle();
        let video_element = VideoElement::new(target_image);
        self.insert(&video_handle, video_element);

        let html_video_element = registry
            .document()
            .create_element("video")
            .inspect_err(|e| warn!("{e:?}"))
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlVideoElement>()
            .inspect_err(|e| warn!("{e:?}"))
            .expect_throw("web_sys::HtmlVideoElement");

        registry.insert(video_handle.id(), html_video_element.clone());

        (video_handle, html_video_element)
    }
}

fn mark_assets_modified(mut video_elements: ResMut<Assets<VideoElement>>) {
    // Mark modified every frame so RenderAsset prepares the texture
    video_elements.iter_mut().for_each(drop);
}

#[allow(clippy::too_many_arguments)]
fn remove_unused_assets(
    mut events: EventReader<AssetEvent<VideoElement>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) {
    for event in events.read() {
        match *event {
            AssetEvent::Removed { id: asset_id } | AssetEvent::Unused { id: asset_id } => {
                registry.remove(asset_id);
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
    let width = element.video_width();
    let height = element.video_height();
    if width == 0 || height == 0 {
        return;
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
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
    if trigger.target() == Entity::PLACEHOLDER
        && let Some(video_element) = video_elements.get(asset_id)
        && let Some(element) = registry.element(asset_id)
    {
        resize_image(video_element, element, &mut images);
    };
}

fn on_canplay(
    trigger: Trigger<ListenerEvent<events::CanPlay>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
    registry: NonSend<VideoElementRegistry>,
) {
    let asset_id = trigger.asset_id();
    if trigger.target() == Entity::PLACEHOLDER
        && let Some(video_element) = video_elements.get(asset_id)
        && let Some(element) = registry.element(asset_id)
    {
        resize_image(video_element, element, &mut images);
    };
}

fn on_resize(
    trigger: Trigger<ListenerEvent<events::Resize>>,
    video_elements: Res<Assets<VideoElement>>,
    mut images: ResMut<Assets<Image>>,
    registry: NonSend<VideoElementRegistry>,
) {
    let asset_id = trigger.asset_id();
    if trigger.target() == Entity::PLACEHOLDER
        && let Some(video_element) = video_elements.get(asset_id)
        && let Some(element) = registry.element(asset_id)
    {
        resize_image(video_element, element, &mut images);
    };
}

fn on_error(
    trigger: Trigger<ListenerEvent<events::Error>>,
    video_elements: Res<Assets<VideoElement>>,
) {
    let asset_id = trigger.asset_id();
    if trigger.target() == Entity::PLACEHOLDER && video_elements.get(asset_id).is_none() {
        warn!("Video asset {:?} failed to load with error", asset_id);
    };
}

fn on_playing(
    trigger: Trigger<ListenerEvent<events::Playing>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) {
    if trigger.target() == Entity::PLACEHOLDER
        && let Some(video_element) = video_elements.get_mut(trigger.asset_id())
    {
        video_element.renderable = true;
    };
}

fn on_ended(
    trigger: Trigger<ListenerEvent<events::Ended>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) {
    if trigger.target() == Entity::PLACEHOLDER
        && let Some(video_element) = video_elements.get_mut(trigger.asset_id())
    {
        video_element.renderable = false;
    };
}
