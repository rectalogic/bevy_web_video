use std::f32::consts::FRAC_PI_4;

use bevy::{
    asset::{AsAssetId, RenderAssetUsages},
    color::palettes::css::GOLD,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::WindowResolution,
};
use bevy_web_video::{
    EventListenerAppExt, EventSender, ListenerEvent, VideoElement, VideoElementAssetsExt,
    VideoElementRegistry, WebVideo, WebVideoError, WebVideoPlugin, events, new_event_type,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(800.0, 600.0),
                ..default()
            }),
            ..default()
        }),
        WebVideoPlugin,
    ))
    .add_listener_event::<CueChange>()
    .add_systems(Startup, setup)
    .add_systems(Update, toggle_mute);

    app.run();
}

new_event_type!(CueChange, "cuechange");

#[derive(Component)]
struct Video;

#[derive(Component)]
struct Caption;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    cuechange_event_sender: Res<EventSender<CueChange>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    let video_image = images.reserve_handle();
    let (video_element_handle, element) = video_elements.new_video(&video_image, &mut registry);
    let video_asset_id = video_element_handle.id();
    let video_entity = commands.spawn(WebVideo::new(video_element_handle)).id();

    element.set_cross_origin(Some("anonymous"));
    element.set_src("https://thepaciellogroup.github.io/AT-browser-tests/video/ElephantsDream.mp4");
    element.set_muted(true); // User must click to unmute
    element.set_loop(true);

    let track = registry
        .document()
        .create_element("track")
        .inspect_err(|e| warn!("{e:?}"))
        .unwrap_throw()
        .dyn_into::<web_sys::HtmlTrackElement>()
        .inspect_err(|e| warn!("{e:?}"))
        .expect_throw("web_sys::HtmlTrackElement");
    track.set_kind("subtitles");
    track.set_srclang("en");
    track.set_src("https://thepaciellogroup.github.io/AT-browser-tests/video/subtitles-en.vtt");
    track.set_default(true);
    element.append_child(&track).map_err(WebVideoError::from)?;

    commands
        .entity(video_entity)
        .observe(loadedmetadata_observer);
    loadedmetadata_event_sender.enable_element_event_observers(
        video_asset_id,
        &element,
        &mut registry,
        video_entity,
    );

    commands.entity(video_entity).observe(cuechange_observer);
    cuechange_event_sender.enable_element_event_observers(
        video_asset_id,
        &track,
        &mut registry,
        video_entity,
    );

    let _ = element.play().map_err(WebVideoError::from)?;

    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 3.0)));

    const CAPTION_X_SCALE: f32 = 1.5;

    let mut image = Image::new_uninit(
        Extent3d {
            width: (512. * CAPTION_X_SCALE) as u32,
            height: 512,
            ..default()
        },
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let caption_image = images.add(image);
    let caption_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: -1, // Render before 3d camera
                target: caption_image.clone().into(),
                ..default()
            },
        ))
        .id();
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(GOLD.into()),
        UiTargetCamera(caption_camera),
        children![(
            Caption,
            Text::default(),
            TextFont {
                font_size: 40.0,
                ..default()
            },
            TextColor::BLACK,
        )],
    ));

    let plane = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)));
    commands.spawn((
        Mesh3d(plane.clone()),
        MeshMaterial3d(materials.add(caption_image)),
        Transform::from_rotation(Quat::from_rotation_y(FRAC_PI_4))
            .with_scale(Vec3::new(CAPTION_X_SCALE, 1.0, 1.0))
            .with_translation(Vec3::new(-0.6, 0.0, 0.0)),
    ));
    commands.spawn((
        Video,
        Mesh3d(plane),
        MeshMaterial3d(materials.add(video_image)),
        Transform::from_rotation(Quat::from_rotation_y(-FRAC_PI_4))
            .with_translation(Vec3::new(0.6, 0.0, 0.0)),
    ));
    commands.spawn(DirectionalLight::default());

    Ok(())
}

fn loadedmetadata_observer(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mut transform: Single<&mut Transform, With<Video>>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id()) {
        let width = element.video_width();
        let height = element.video_height();
        let aspect = width as f32 / height as f32;
        // Scale plane to match video aspect ratio.
        transform.scale = Vec3::new(aspect.max(1.0), aspect.min(1.0), 1.0);
    }
}

fn cuechange_observer(
    trigger: Trigger<ListenerEvent<CueChange>>,
    mut text: Single<&mut Text, With<Caption>>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id())
        && let Some(text_tracks) = element.text_tracks()
        && let Some(track) = text_tracks.get(0)
        && let Some(cues) = track.active_cues()
        && let Some(cue) = cues.get(0)
    {
        text.0 = cue.text();
    }
}

fn toggle_mute(
    mouse_button: Res<ButtonInput<MouseButton>>,
    video: Single<&WebVideo>,
    registry: NonSend<VideoElementRegistry>,
) {
    if mouse_button.just_pressed(MouseButton::Left)
        && let Some(element) = registry.element(video.as_asset_id())
    {
        element.set_muted(!element.muted());
    }
}
