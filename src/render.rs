use crate::registry::Registry;
use bevy::{
    prelude::*,
    render::{render_asset::RenderAssets, renderer::RenderQueue, texture::GpuImage},
};
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub fn render_videos(queue: Res<RenderQueue>, images: Res<RenderAssets<GpuImage>>) {
    Registry::with_borrow(|registry| {
        registry
            .iter()
            .filter_map(|video_element| {
                if video_element.is_loaded() {
                    images
                        .get(video_element.target_texture_id())
                        .map(|gpu_image| (gpu_image, video_element))
                } else {
                    None
                }
            })
            .for_each(|(gpu_image, video_element)| {
                queue.copy_external_image_to_texture(
                    &CopyExternalImageSourceInfo {
                        source: ExternalImageSource::HTMLVideoElement(video_element.element()),
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
            });
    });
}
