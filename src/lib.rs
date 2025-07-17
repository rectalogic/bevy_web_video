use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Extract, MainWorld, Render, RenderApp, RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
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
        app.add_plugins(ExtractComponentPlugin::<WebVideo>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .world_mut()
            .insert_non_send_resource(VideoElements);
        render_app
            .add_systems(ExtractSchedule, extract_videos)
            .add_systems(Render, render_videos.in_set(RenderSet::PrepareResources));
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Component, ExtractComponent)]
pub struct WebVideo {
    url: String,
    image_id: AssetId<Image>,
}

struct VideoElement {
    element: HtmlVideoElement,
    initialized: bool,
}

#[derive(Default, Deref, DerefMut)]
struct VideoElements(HashMap<WebVideo, VideoElement>);

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

fn extract_videos(
    mut main_world: ResMut<MainWorld>,
    videos: Extract<Query<&WebVideo>>,
    mut video_elements: NonSendMut<VideoElements>,
) -> Result<()> {
    let Some(mut images) = main_world.get_resource_mut::<Assets<Image>>() else {
        return Err("Assets<Image> not found".into());
    };
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");
    for video in videos.iter() {
        match video_elements.get_mut(video) {
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
                    video.clone(),
                    VideoElement {
                        element: video_element,
                        initialized: false,
                    },
                );
            }
        }
    }
    Ok(())
}

fn render_videos(
    // videos: Query<&WebVideo>,
    video_elements: NonSend<VideoElements>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
) {
    for (video, video_element) in video_elements.iter() {
        let Some(gpu_image) = images.get(video.image_id) else {
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
