#[cfg(feature = "webgpu")]
use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
};
use bevy::{math::Affine2, prelude::*, window::WindowResolution};
use bevy_web_video::{
    EventSender, EventWithAssetId, ListenerEvent, VideoElement, VideoElementCreated,
    VideoElementRegistry, WebVideo, WebVideoError, WebVideoPlugin, events,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() {
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
    .add_systems(Startup, setup)
    .add_systems(Update, update);

    app.run();
}

#[derive(Component)]
struct Animated;

#[derive(Component)]
struct VideoA;

#[cfg(feature = "webgpu")]
#[derive(Component)]
struct VideoB;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(feature = "webgpu")] mut decal_materials: ResMut<
        Assets<ForwardDecalMaterial<StandardMaterial>>,
    >,
    images: Res<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) -> Result<()> {
    let image_handle1 = images.reserve_handle();
    let video_element_handle1 = video_elements.add(VideoElement::new(&image_handle1));

    let mut video_commands = commands.spawn(WebVideo::new(video_element_handle1));

    video_commands.observe(video1_created_observer);

    commands.spawn((
        Animated,
        VideoA,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(image_handle1.clone()),
            ..default()
        })),
        Transform::from_xyz(-0.75, 0.0, 0.0),
    ));

    // Decals broken on webgl2 https://github.com/bevyengine/bevy/issues/19177
    #[cfg(feature = "webgpu")]
    {
        let image_handle2 = images.reserve_handle();
        let video_element_handle2 = video_elements.add(VideoElement::new(&image_handle2));

        commands
            .spawn(WebVideo::new(video_element_handle2))
            .observe(video2_created_observer);

        let decal_material1 = decal_materials.add(new_decal_material(image_handle1));
        let decal_material2 = decal_materials.add(new_decal_material(image_handle2));

        commands.spawn((
            Animated,
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.75, 0.0, 0.0),
            children![
                // Top
                (
                    VideoA,
                    ForwardDecal,
                    MeshMaterial3d(decal_material1.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                ),
                // Bottom
                (
                    VideoB,
                    ForwardDecal,
                    MeshMaterial3d(decal_material2.clone()),
                    Transform::from_xyz(0.0, -0.5, 0.0)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Y)),
                ),
                // Left
                (
                    VideoB,
                    ForwardDecal,
                    MeshMaterial3d(decal_material2.clone()),
                    Transform::from_xyz(-0.5, 0.0, 0.0)
                        .with_rotation(Quat::from_rotation_arc(-Vec3::X, -Vec3::Y)),
                ),
                // Right
                (
                    VideoA,
                    ForwardDecal,
                    MeshMaterial3d(decal_material1.clone()),
                    Transform::from_xyz(0.5, 0.0, 0.0)
                        .with_rotation(Quat::from_rotation_arc(Vec3::X, -Vec3::Y)),
                ),
                // Front
                (
                    VideoA,
                    ForwardDecal,
                    MeshMaterial3d(decal_material1.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.5)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
                ),
                // Back
                (
                    VideoB,
                    ForwardDecal,
                    MeshMaterial3d(decal_material2.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.5)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Z)),
                ),
            ],
        ));
    }

    commands.spawn((PointLight::default(), Transform::from_xyz(3.0, 3.0, 4.0)));
    commands.spawn((
        Camera3d::default(),
        #[cfg(feature = "webgpu")]
        DepthPrepass, // required for decals
        #[cfg(feature = "webgpu")]
        Msaa::Off, // workaround https://github.com/bevyengine/bevy/issues/19177
        Transform::from_xyz(0., 0., 3.),
    ));
    Ok(())
}

fn video1_created_observer(
    trigger: Trigger<VideoElementCreated>,
    mut commands: Commands,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    let mut video_commands = commands.entity(trigger.target());
    if let Some(element) = registry.element(trigger.asset_id()) {
        loadedmetadata_event_sender.add_video_event_listener(
            trigger.asset_id(),
            &element,
            &mut registry,
            &mut video_commands,
            scale_cube_listener,
        );

        #[cfg(feature = "webgpu")]
        loadedmetadata_event_sender.add_video_event_listener(
            trigger.asset_id(),
            &element,
            &mut registry,
            &mut video_commands,
            scale_decals_listener::<VideoA>,
        );

        element.set_cross_origin(Some("anonymous"));
        element.set_src("https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/bbb_sunflower_1080p_30fps_normal_10min.mp4");
        element.set_muted(true);
        element.set_loop(true);
        let _ = element.play().map_err(WebVideoError::from)?;

        Ok(())
    } else {
        Err("missing video".into())
    }
}

fn video2_created_observer(
    trigger: Trigger<VideoElementCreated>,
    mut commands: Commands,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    let mut video_commands = commands.entity(trigger.target());
    if let Some(element) = registry.element(trigger.asset_id()) {
        loadedmetadata_event_sender.add_video_event_listener(
            trigger.asset_id(),
            &element,
            &mut registry,
            &mut video_commands,
            scale_decals_listener::<VideoB>,
        );

        element.set_cross_origin(Some("anonymous"));
        element.set_src(
            "https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/elephants_dream_1280x720.mp4"
        );
        element.set_muted(true);
        element.set_loop(true);
        let _ = element.play().map_err(WebVideoError::from)?;
        Ok(())
    } else {
        Err("missing video".into())
    }
}

fn scale_cube_listener(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mesh_material: Single<&MeshMaterial3d<StandardMaterial>, With<VideoA>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id())
        && let Some(material) = materials.get_mut(&mesh_material.0)
    {
        let width = element.video_width();
        let height = element.video_height();

        // Scale uv transform to match video aspect ratio.
        // Zoom in so video fills the face.
        if width > height {
            let aspect = height as f32 / width as f32;
            material.uv_transform = Affine2::from_translation(Vec2::new((1.0 - aspect) / 2.0, 0.0))
                * Affine2::from_scale(Vec2::new(aspect, 1.0));
        } else {
            let aspect = width as f32 / height as f32;
            material.uv_transform = Affine2::from_translation(Vec2::new(0.0, (1.0 - aspect) / 2.0))
                * Affine2::from_scale(Vec2::new(1.0, aspect));
        }
    }
}

#[cfg(feature = "webgpu")]
fn new_decal_material(image: Handle<Image>) -> ForwardDecalMaterial<StandardMaterial> {
    ForwardDecalMaterial {
        base: StandardMaterial {
            base_color_texture: Some(image),
            ..default()
        },
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: 1.0,
        },
    }
}

#[cfg(feature = "webgpu")]
fn scale_decals_listener<V: Component>(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mut decals: Query<&mut Transform, (With<ForwardDecal>, With<V>)>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id()) {
        let width = element.video_width();
        let height = element.video_height();
        for mut transform in &mut decals {
            // Scale decal to match video aspect ratio
            if width > height {
                transform.scale.z = height as f32 / width as f32;
            } else {
                transform.scale.x = width as f32 / height as f32;
            }
        }
    }
}

fn update(mut videos: Query<&mut Transform, With<Animated>>, time: Res<Time>) {
    for mut transform in videos.iter_mut() {
        transform.rotate_x(time.delta_secs() * 0.8);
        transform.rotate_z(time.delta_secs() * 0.25);
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
