#[cfg(feature = "webgpu")]
use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
};
use bevy::{math::Affine2, prelude::*, window::WindowResolution};
use bevy_web_video::{
    EntityAddEventListenerExt, ListenerEvent, VideoId, WebVideo, WebVideoError, WebVideoPlugin,
    event,
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(feature = "webgpu")] mut decal_materials: ResMut<
        Assets<ForwardDecalMaterial<StandardMaterial>>,
    >,
    mut images: ResMut<Assets<Image>>,
) -> Result<()> {
    let image_handle = images.add(VideoId::new_image());
    let video_id = VideoId::new(&image_handle);
    let video = WebVideo::create_video_element(video_id)?;
    video.set_cross_origin(Some("anonymous"));
    video.set_src("https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/bbb_sunflower_1080p_30fps_normal_10min.mp4");
    video.set_muted(true);
    video.set_loop(true);
    let _ = video.play().map_err(WebVideoError::from)?;

    commands
        .spawn((
            Animated,
            WebVideo::new(video_id),
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(image_handle.clone()),
                ..default()
            })),
            Transform::from_xyz(-0.75, 0.0, 0.0),
        ))
        .add_event_listener(
            video_id,
            |trigger: Trigger<ListenerEvent<event::LoadedMetadata>>,
             mut videos: Query<(&WebVideo, &MeshMaterial3d<StandardMaterial>)>,
             mut materials: ResMut<Assets<StandardMaterial>>| {
                if let Ok((web_video, mesh_material)) = videos.get_mut(trigger.target())
                    && let Some(material) = materials.get_mut(mesh_material)
                {
                    let element = web_video.video_element();
                    let width = element.video_width();
                    let height = element.video_height();

                    // Scale uv transform to match video aspect ratio.
                    // Zoom in so video fills the face.
                    if width > height {
                        let aspect = height as f32 / width as f32;
                        material.uv_transform =
                            Affine2::from_translation(Vec2::new((1.0 - aspect) / 2.0, 0.0))
                                * Affine2::from_scale(Vec2::new(aspect, 1.0));
                    } else {
                        let aspect = width as f32 / height as f32;
                        material.uv_transform =
                            Affine2::from_translation(Vec2::new(0.0, (1.0 - aspect) / 2.0))
                                * Affine2::from_scale(Vec2::new(1.0, aspect));
                    }
                }
            },
        );

    // Decals broken on webgl2 https://github.com/bevyengine/bevy/issues/19177
    #[cfg(feature = "webgpu")]
    {
        let video_id1 = video_id;
        let image_handle1 = image_handle;
        let image_handle2 = images.add(VideoId::new_image());
        let video_id2 = VideoId::new(&image_handle2);
        let video = WebVideo::create_video_element(video_id2)?;
        video.set_cross_origin(Some("anonymous"));
        video.set_src(
            "https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/elephants_dream_1280x720.mp4"
        );
        video.set_muted(true);
        video.set_loop(true);
        let _ = video.play().map_err(WebVideoError::from)?;

        let parent = commands
            .spawn((
                Animated,
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(Color::WHITE)),
                Transform::from_xyz(0.75, 0.0, 0.0),
            ))
            .id();

        let decal_material1 = decal_materials.add(new_decal_material(image_handle1));
        let decal_material2 = decal_materials.add(new_decal_material(image_handle2));

        let decals = [
            // Top
            video_decal(
                &mut commands,
                video_id1,
                decal_material1.clone(),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ),
            // Bottom
            video_decal(
                &mut commands,
                video_id2,
                decal_material2.clone(),
                Transform::from_xyz(0.0, -0.5, 0.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Y)),
            ),
            // Left
            video_decal(
                &mut commands,
                video_id2,
                decal_material2.clone(),
                Transform::from_xyz(-0.5, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_arc(-Vec3::X, -Vec3::Y)),
            ),
            // Right
            video_decal(
                &mut commands,
                video_id1,
                decal_material1.clone(),
                Transform::from_xyz(0.5, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_arc(Vec3::X, -Vec3::Y)),
            ),
            // Front
            video_decal(
                &mut commands,
                video_id1,
                decal_material1,
                Transform::from_xyz(0.0, 0.0, 0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
            ),
            // Back
            video_decal(
                &mut commands,
                video_id2,
                decal_material2,
                Transform::from_xyz(0.0, 0.0, -0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, -Vec3::Z)),
            ),
        ];
        commands.entity(parent).add_children(&decals);
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
fn video_decal(
    commands: &mut Commands,
    video_id: VideoId,
    decal_material: Handle<ForwardDecalMaterial<StandardMaterial>>,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            WebVideo::new(video_id),
            ForwardDecal,
            MeshMaterial3d(decal_material),
            transform,
        ))
        .add_event_listener(
            video_id,
            |trigger: Trigger<ListenerEvent<event::LoadedMetadata>>,
             mut decals: Query<(&WebVideo, &mut Transform), With<ForwardDecal>>| {
                if let Ok((web_video, mut transform)) = decals.get_mut(trigger.target()) {
                    let element = web_video.video_element();
                    let width = element.video_width();
                    let height = element.video_height();

                    // Scale decal to match video aspect ratio
                    if width > height {
                        transform.scale.z = height as f32 / width as f32;
                    } else {
                        transform.scale.x = width as f32 / height as f32;
                    }
                }
            },
        )
        .id()
}

fn update(mut videos: Query<&mut Transform, With<Animated>>, time: Res<Time>) {
    for mut transform in videos.iter_mut() {
        transform.rotate_x(time.delta_secs() * 0.8);
        transform.rotate_z(time.delta_secs() * 0.25);
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
