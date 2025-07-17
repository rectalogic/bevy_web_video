use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    ecs::{component::HookContext, world::DeferredWorld},
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderQueue,
        texture::GpuImage,
    },
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlVideoElement;
use wgpu_types::{
    CopyExternalImageDestInfo, CopyExternalImageSourceInfo, ExternalImageSource, Origin2d,
    Origin3d, PredefinedColorSpace, TextureAspect,
};

pub struct WebVideoPlugin;

impl Plugin for WebVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RemoveWebVideo>()
            .add_event::<AddWebVideo>()
            .init_non_send_resource::<VideoElements>()
            .add_systems(Update, (handle_play_tasks, update_video_elements));
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

#[derive(Component)]
#[component(on_add = add_webvideo_hook, on_despawn = remove_webvideo_hook, on_remove = remove_webvideo_hook, on_replace = replace_webvideo_hook)]
pub struct WebVideo {
    url: String,
    image_id: AssetId<Image>,
}

#[derive(Component)]
pub struct PlayVideoTask(Task<Result<()>>);

#[derive(Event)]
struct AddWebVideo(Entity);

#[derive(Event)]
struct RemoveWebVideo(Entity);

#[derive(Clone)]
struct VideoElement {
    element: HtmlVideoElement,
    image_id: AssetId<Image>,
    playing: bool,
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

fn add_webvideo_hook(mut world: DeferredWorld, context: HookContext) {
    world.send_event(AddWebVideo(context.entity));
}

fn remove_webvideo_hook(mut world: DeferredWorld, context: HookContext) {
    world.send_event(RemoveWebVideo(context.entity));
}

fn replace_webvideo_hook(mut world: DeferredWorld, context: HookContext) {
    world.send_event(RemoveWebVideo(context.entity));
    world.send_event(AddWebVideo(context.entity));
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
    Ok(video_element)
}

fn update_video_elements(
    mut commands: Commands,
    mut videos_removed: EventReader<RemoveWebVideo>,
    mut videos_added: EventReader<AddWebVideo>,
    videos: Query<&WebVideo>,
    mut video_elements: NonSendMut<VideoElements>,
) {
    for event in videos_removed.read() {
        commands
            .entity(event.0)
            .remove::<(WebVideo, PlayVideoTask)>();
        video_elements.remove(&event.0);
    }
    if videos_added.is_empty() {
        return;
    }
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");
    for event in videos_added.read() {
        let Ok(video) = videos.get(event.0) else {
            continue;
        };
        let Ok(video_element) = create_video(&document, &video.url)
            .inspect_err(|err| warn!("Failed to create video: {err}"))
        else {
            continue;
        };
        let Ok(promise) = video_element
            .play()
            .inspect_err(|err| warn!("Failed to play video: {err:?}"))
        else {
            continue;
        };
        let task = AsyncComputeTaskPool::get().spawn(async move {
            match wasm_bindgen_futures::JsFuture::from(promise).await {
                Ok(_) => Ok(()),
                Err(err) => Err(format!("Failed to play video: {err:?}").into()),
            }
        });
        commands.entity(event.0).insert(PlayVideoTask(task));
        video_elements.insert(
            event.0,
            VideoElement {
                element: video_element,
                image_id: video.image_id,
                playing: false,
            },
        );
    }
}

fn handle_play_tasks(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut play_tasks: Query<(Entity, &mut PlayVideoTask)>,
    mut video_elements: NonSendMut<VideoElements>,
) {
    for (entity, mut task) in &mut play_tasks {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            match result {
                Ok(_) => {
                    if let Some(video_element) = video_elements.get_mut(&entity)
                        && let Some(image) = images.get_mut(video_element.image_id)
                    {
                        image.texture_descriptor.size.width = video_element.element.video_width();
                        image.texture_descriptor.size.height = video_element.element.video_height();
                        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
                        video_element.playing = true;
                    }
                }
                Err(err) => {
                    warn!("{err}");
                    video_elements.remove(&entity);
                }
            }
            commands.entity(entity).remove::<PlayVideoTask>();
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
        if !video_element.playing {
            continue;
        };
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
