use crate::VideoElement;
use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub struct WebVideoRenderPlugin;

impl Plugin for WebVideoRenderPlugin {
    fn build(&self, app: &mut App) {
        // Render videos after GpuImage is prepared
        app.add_plugins(RenderAssetPlugin::<RenderVideoElement, GpuImage>::default());
    }
}

struct RenderVideoElement;

impl RenderAsset for RenderVideoElement {
    type SourceAsset = VideoElement;
    type Param = (SRes<RenderQueue>, SRes<RenderAssets<GpuImage>>);

    fn prepare_asset(
        video_element: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_queue, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        if video_element.is_renderable()
            && let Some(gpu_image) = gpu_images.get(video_element.target_image_id())
            && let Some(element) = video_element.element()
        {
            render_queue.copy_external_image_to_texture(
                &CopyExternalImageSourceInfo {
                    source: ExternalImageSource::HTMLVideoElement(element),
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
            Ok(RenderVideoElement)
        } else {
            Err(PrepareAssetError::RetryNextUpdate(video_element))
        }
    }
}
