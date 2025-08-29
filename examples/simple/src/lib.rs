use bevy::{prelude::*, window::WindowResolution};
use bevy_web_video::{
    EventWithAssetId, VideoElement, VideoElementCreated, VideoElementRegistry, WebVideo,
    WebVideoError, WebVideoPlugin,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1280.0, 720.0),
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
) {
    let image = images.reserve_handle();
    let video_element = video_elements.add(VideoElement::new(&image));

    commands
        .spawn(WebVideo::new(video_element))
        .observe(observe_video_created);
    commands.spawn(Sprite::from_image(image));
    commands.spawn(Camera2d);
}

fn observe_video_created(
    trigger: Trigger<VideoElementCreated>,
    registry: NonSend<VideoElementRegistry>,
) -> Result<()> {
    let asset_id = trigger.asset_id();
    if let Some(element) = registry.element(asset_id) {
        element.set_cross_origin(Some("anonymous"));
        element.set_src(
            "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
        );
        element.set_muted(true);
        element.set_loop(true);
        let _ = element.play().map_err(WebVideoError::from)?;
    }
    Ok(())
}
