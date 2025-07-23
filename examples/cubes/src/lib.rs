use bevy::{prelude::*, window::WindowResolution};
use bevy_web_video::{WebVideoError, WebVideoPlugin, WebVideoRegistry};
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

fn setup(
    mut commands: Commands,
    video_registry: Res<WebVideoRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
) -> Result<()> {
    let (image_handle, video) = video_registry.new_video_texture(images)?;
    video.set_cross_origin(Some("anonymous"));
    video.set_src("https://cdn.glitch.me/364f8e5a-f12f-4f82-a386-20e6be6b1046/bbb_sunflower_1080p_30fps_normal_10min.mp4");
    video.set_muted(true);
    video.set_loop(true);
    let _ = video.play().map_err(WebVideoError::from)?;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(image_handle)),
    ));
    commands.spawn((PointLight::default(), Transform::from_xyz(3.0, 3.0, 2.0)));
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0., 2.)));
    Ok(())
}

fn update(mut videos: Query<&mut Transform, With<Mesh3d>>, time: Res<Time>) {
    for mut transform in videos.iter_mut() {
        transform.rotate_x(time.delta_secs() * 0.8);
        transform.rotate_z(time.delta_secs() * 0.25);
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
