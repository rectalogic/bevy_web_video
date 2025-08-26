use crate::{
    VideoElement,
    registry::{ElementRegistry, RegistryId},
};
use bevy::{
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSet, render_asset::RenderAssets, renderer::RenderQueue,
        texture::GpuImage,
    },
};
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub struct WebVideoRenderPlugin;

impl Plugin for WebVideoRenderPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<RenderableVideos>()
            .add_systems(ExtractSchedule, extract_videos)
            .add_systems(Render, render_videos.in_set(RenderSet::PrepareResources));
    }
}

// This should be NonSend and contain the actual web_sys elements,
// but can't use NonSend resource in a SubApp
#[derive(Resource, Default)]
struct RenderableVideos(Vec<RenderableVideo>);

struct RenderableVideo {
    image_id: AssetId<Image>,
    registry_id: RegistryId,
}

fn extract_videos(
    mut renderable_videos: ResMut<RenderableVideos>,
    video_elements: Extract<Res<Assets<VideoElement>>>,
) {
    let videos: Vec<RenderableVideo> = video_elements
        .iter()
        .filter_map(|(_, video_element)| {
            if video_element.is_renderable() {
                Some(RenderableVideo {
                    image_id: video_element.target_texture().id(),
                    registry_id: video_element.registry_id(),
                })
            } else {
                None
            }
        })
        .collect();
    renderable_videos.0 = videos;
}

fn render_videos(
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
    renderable_videos: Res<RenderableVideos>,
) {
    ElementRegistry::with_borrow(|registry| {
        for video in &renderable_videos.0 {
            if let Some(gpu_image) = images.get(video.image_id)
                && let Some(element) = registry.get(&video.registry_id)
            {
                queue.copy_external_image_to_texture(
                    &CopyExternalImageSourceInfo {
                        source: ExternalImageSource::HTMLVideoElement(element.element().clone()),
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
    });
}
