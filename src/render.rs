use crate::{VideoElement, VideoElementRegistry};
use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, RenderApp,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        renderer::RenderQueue,
        texture::GpuImage,
    },
};
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub struct VideoRenderPlugin;

impl Plugin for VideoRenderPlugin {
    fn build(&self, app: &mut App) {
        // Render videos after GpuImage is prepared
        app.add_plugins(RenderAssetPlugin::<RenderVideoElement, GpuImage>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(ExtractSchedule, extract_elements)
            .world_mut()
            .init_non_send_resource::<RenderElements>();
    }
}

#[derive(Default, Deref, DerefMut)]
struct RenderElements(HashMap<AssetId<VideoElement>, web_sys::HtmlVideoElement>);

fn extract_elements(
    registry: Extract<NonSend<VideoElementRegistry>>,
    video_elements: Extract<Res<Assets<VideoElement>>>,
    mut render_elements: NonSendMut<RenderElements>,
) {
    for (asset_id, video_element) in video_elements.iter() {
        if video_element.is_renderable()
            && let Some(element) = registry.element(asset_id)
        {
            render_elements.insert(asset_id, element);
        }
    }
}

struct RenderVideoElement;

impl RenderAsset for RenderVideoElement {
    type SourceAsset = VideoElement;
    type Param = (
        SRes<RenderQueue>,
        SRes<RenderAssets<GpuImage>>,
        NonSendMut<'static, RenderElements>,
    );

    fn prepare_asset(
        video_element: Self::SourceAsset,
        asset_id: AssetId<Self::SourceAsset>,
        (render_queue, gpu_images, render_elements): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        if let Some(gpu_image) = gpu_images.get(video_element.target_image_id())
            && let Some(element) = render_elements.remove(&asset_id)
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
            // Marker asset, we already did the work above
            Ok(RenderVideoElement)
        } else {
            Err(PrepareAssetError::RetryNextUpdate(video_element))
        }
    }
}
