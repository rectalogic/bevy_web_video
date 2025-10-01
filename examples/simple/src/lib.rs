use bevy::{prelude::*, window::WindowResolution};
use bevy_web_video::{
    VideoElement, VideoElementAssetsExt, VideoElementRegistry, WebVideo, WebVideoError,
    WebVideoPlugin,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }),
        WebVideoPlugin,
    ))
    .add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    let image_handle = images.reserve_handle();
    let (video_element_handle, element) = video_elements.new_video(&image_handle, &mut registry);
    element.set_cross_origin(Some("anonymous"));
    element.set_src(
        "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
    );
    element.set_muted(true);
    element.set_loop(true);
    let _ = element.play().map_err(WebVideoError::from)?;
    commands.spawn(WebVideo::new(video_element_handle));
    commands.spawn(Sprite::from_image(image_handle));
    commands.spawn(Camera2d);
    Ok(())
}
