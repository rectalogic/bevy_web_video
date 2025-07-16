use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderQueue,
        texture::GpuImage,
    },
    window::WindowResolution,
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlVideoElement;
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

#[wasm_bindgen]
pub fn start(video: HtmlVideoElement) {
    console_error_panic_hook::set_once();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some("#bevy-canvas".into()),
                // https://github.com/bevyengine/bevy/issues/20164
                resolution: WindowResolution::new(800.0, 800.0),
                ..default()
            }),
            ..default()
        }),
        ExtractComponentPlugin::<WebVideo>::default(),
    ))
    .insert_non_send_resource(video.clone())
    .add_systems(Startup, setup);

    let render_app = app.sub_app_mut(RenderApp);
    render_app.world_mut().insert_non_send_resource(video);
    render_app.add_systems(Render, render_video.in_set(RenderSet::PrepareResources));

    app.run();
}

#[derive(Clone, Component, ExtractComponent)]
struct WebVideo {
    pub image_id: AssetId<Image>,
    pub size: Extent3d,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    video_element: NonSend<HtmlVideoElement>,
) {
    let size = Extent3d {
        width: video_element.video_width(),
        height: video_element.video_height(),
        ..default()
    };
    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);
    commands.spawn((
        WebVideo {
            image_id: image_handle.id(),
            size,
        },
        Sprite::from_image(image_handle),
    ));
    commands.spawn(Camera2d);
}

fn render_video(
    videos: Query<&WebVideo>,
    video_element: NonSend<HtmlVideoElement>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<GpuImage>>,
) {
    for video in videos.iter() {
        let Some(gpu_image) = images.get(video.image_id) else {
            continue;
        };
        queue.copy_external_image_to_texture(
            &CopyExternalImageSourceInfo {
                source: ExternalImageSource::HTMLVideoElement(video_element.clone()),
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
            video.size,
        );
    }
}
