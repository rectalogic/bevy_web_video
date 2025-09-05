// Decals broken on webgl2 https://github.com/bevyengine/bevy/issues/19177

use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
};
use bevy::{prelude::*, window::WindowResolution};
use bevy_web_video::{
    EventSender, ListenerEvent, VideoElement, VideoElementAssetsExt, VideoElementRegistry,
    WebVideo, WebVideoError, WebVideoPlugin, events,
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
    .add_systems(Startup, setup)
    .add_systems(Update, update);

    app.run();
}

#[derive(Component)]
struct SpinCube;

#[derive(Component)]
struct TumbleCube;

#[derive(Component)]
struct DecalMaterial1;

#[derive(Component)]
struct DecalMaterial2;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    images: Res<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    let image_handle1 = images.reserve_handle();
    let (video_element_handle1, element1) = video_elements.new_video(&image_handle1, &mut registry);
    let video_element_id1 = video_element_handle1.id();
    let video_entity1 = commands.spawn(WebVideo::new(video_element_handle1)).id();

    commands
        .entity(video_entity1)
        .observe(scale_spincube_listener)
        .observe(scale_decals_listener::<DecalMaterial1>);

    loadedmetadata_event_sender.enable_element_event_observers(
        video_element_id1,
        &element1,
        &mut registry,
        video_entity1,
    );

    element1.set_cross_origin(Some("anonymous"));
    element1.set_src("https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/bbb_sunflower_1080p_30fps_normal_10min.mp4");
    element1.set_muted(true);
    element1.set_loop(true);
    let _ = element1.play().map_err(WebVideoError::from)?;

    commands.spawn((
        SpinCube,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(image_handle1.clone()),
            ..default()
        })),
        Transform::from_xyz(-0.75, 0.0, 0.0),
    ));

    let image_handle2 = images.reserve_handle();
    let (video_element_handle2, element2) = video_elements.new_video(&image_handle2, &mut registry);
    let video_element_id2 = video_element_handle2.id();

    let video_entity2 = commands.spawn(WebVideo::new(video_element_handle2)).id();

    commands
        .entity(video_entity2)
        .observe(scale_decals_listener::<DecalMaterial2>);
    loadedmetadata_event_sender.enable_element_event_observers(
        video_element_id2,
        &element2,
        &mut registry,
        video_entity2,
    );

    element2.set_cross_origin(Some("anonymous"));
    element2.set_src(
        "https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/elephants_dream_1280x720.mp4",
    );
    element2.set_muted(true);
    element2.set_loop(true);
    let _ = element2.play().map_err(WebVideoError::from)?;

    let decal_material1 = decal_materials.add(new_decal_material(image_handle1));
    let decal_material2 = decal_materials.add(new_decal_material(image_handle2));

    commands.spawn((
        TumbleCube,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(1.0, 0.0, 0.0),
        children![
            // Top
            (
                DecalMaterial1,
                ForwardDecal,
                MeshMaterial3d(decal_material1.clone()),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ),
            // Bottom
            (
                DecalMaterial2,
                ForwardDecal,
                MeshMaterial3d(decal_material2.clone()),
                Transform::from_xyz(0.0, -0.5, 0.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Y)),
            ),
            // Left
            (
                DecalMaterial2,
                ForwardDecal,
                MeshMaterial3d(decal_material2.clone()),
                Transform::from_xyz(-0.5, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_arc(-Vec3::X, -Vec3::Y)),
            ),
            // Right
            (
                DecalMaterial1,
                ForwardDecal,
                MeshMaterial3d(decal_material1.clone()),
                Transform::from_xyz(0.5, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::X, -Vec3::Y)),
            ),
            // Front
            (
                DecalMaterial1,
                ForwardDecal,
                MeshMaterial3d(decal_material1.clone()),
                Transform::from_xyz(0.0, 0.0, 0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
            ),
            // Back
            (
                DecalMaterial2,
                ForwardDecal,
                MeshMaterial3d(decal_material2.clone()),
                Transform::from_xyz(0.0, 0.0, -0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Z)),
            ),
        ],
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(3.0, 3.0, 4.0)));
    commands.spawn((
        Camera3d::default(),
        DepthPrepass, // required for decals
        Msaa::Off,    // workaround https://github.com/bevyengine/bevy/issues/19177
        Transform::from_xyz(0., 0., 3.5),
    ));
    Ok(())
}

fn scale_spincube_listener(
    trigger: Trigger<ListenerEvent<events::LoadedMetadata>>,
    mut transform: Single<&mut Transform, With<SpinCube>>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id()) {
        let width = element.video_width();
        let height = element.video_height();
        let aspect = width as f32 / height as f32;
        // Scale cube to match video aspect ratio.
        transform.scale = Vec3::new(aspect.max(1.0), aspect.min(1.0), 1.0);
    }
}

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

fn update(
    mut tumble_videos: Query<&mut Transform, (With<TumbleCube>, Without<SpinCube>)>,
    mut spin_videos: Query<&mut Transform, (With<SpinCube>, Without<TumbleCube>)>,
    time: Res<Time>,
) {
    for mut transform in tumble_videos.iter_mut() {
        transform.rotate_x(time.delta_secs() * 0.8);
        transform.rotate_z(time.delta_secs() * 0.25);
        transform.rotate_y(time.delta_secs() * 0.5);
    }
    for mut transform in spin_videos.iter_mut() {
        transform.rotate_x(time.delta_secs() * 0.4);
    }
}
