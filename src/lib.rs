use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlVideoElement;
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

//XXX need to remove entry from HashMap when WebVideo component is removed/despawned

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<VideoElements>()
            .add_systems(Update, update_video_elements);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .world_mut()
            .init_non_send_resource::<RenderVideoElements>();
        render_app
            .add_systems(ExtractSchedule, extract_video_elements)
            .add_systems(Render, render_videos.in_set(RenderSet::PrepareResources));
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Component)]
pub struct WebVideo {
    url: String,
    image_id: AssetId<Image>,
}

#[derive(Clone)]
struct VideoElement {
    element: HtmlVideoElement,
    image_id: AssetId<Image>,
    initialized: bool,
}

#[derive(Default, Clone, Deref, DerefMut)]
struct VideoElements(HashMap<Entity, VideoElement>);

#[derive(Default, Clone, Deref, DerefMut)]
struct RenderVideoElements(Vec<VideoElement>);

impl WebVideo {
    pub fn new_with_image(url: &str, mut images: ResMut<Assets<Image>>) -> (Self, Handle<Image>) {
        let mut image = Image::new_uninit(
            Extent3d::default(),
            TextureDimension::D2,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        let image_handle = images.add(image);
        (
            Self {
                url: url.into(),
                image_id: image_handle.id(),
            },
            image_handle,
        )
    }
}

fn create_video(document: &web_sys::Document, url: &str) -> Result<HtmlVideoElement> {
    let video_element = document
        .create_element("video")
        .map_err(|err| format!("{err:?}"))?
        .dyn_into::<web_sys::HtmlVideoElement>()
        .map_err(|err| format!("{err:?}"))?;
    video_element.set_cross_origin(Some("anonymous"));
    video_element.set_src(url);
    video_element.set_muted(true);
    //XXX await the Promise?
    let _ = video_element.play().unwrap();
    Ok(video_element)
}

fn update_video_elements(
    videos: Query<(Entity, &WebVideo)>,
    mut images: ResMut<Assets<Image>>,
    mut video_elements: NonSendMut<VideoElements>,
) {
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");
    for (entity, video) in videos.iter() {
        match video_elements.get_mut(&entity) {
            Some(video_element) => {
                if !video_element.initialized
                    && video_element.element.ready_state()
                        >= web_sys::HtmlMediaElement::HAVE_METADATA
                {
                    if let Some(image) = images.get_mut(video.image_id) {
                        image.texture_descriptor.size.width = video_element.element.video_width();
                        image.texture_descriptor.size.height = video_element.element.video_height();
                        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
                        video_element.initialized = true;
                    }
                }
            }
            None => {
                let video_element = match create_video(&document, &video.url) {
                    Ok(video_element) => video_element,
                    Err(err) => {
                        warn!("Failed to create video: {err}");
                        continue;
                    }
                };
                video_elements.insert(
                    entity,
                    VideoElement {
                        element: video_element,
                        image_id: video.image_id,
                        initialized: false,
                    },
                );
            }
        }
    }
}

fn extract_video_elements(
    video_elements: Extract<NonSend<VideoElements>>,
    mut render_video_elements: NonSendMut<RenderVideoElements>,
) {
    render_video_elements.0 = video_elements.values().cloned().collect();
}

fn render_videos(
    render_video_elements: NonSend<RenderVideoElements>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
) {
    for video_element in render_video_elements.iter() {
        let Some(gpu_image) = images.get(video_element.image_id) else {
            continue;
        };
        queue.copy_external_image_to_texture(
            &CopyExternalImageSourceInfo {
                source: ExternalImageSource::HTMLVideoElement(video_element.element.clone()),
                origin: Origin2d::ZERO,
                flip_y: false,
            },
            CopyExternalImageDestInfo {
                texture: &gpu_image.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
                color_space: PredefinedColorSpace::Srgb,
                premultiplied_alpha: true,
            },
            gpu_image.size,
        );
    }
}
