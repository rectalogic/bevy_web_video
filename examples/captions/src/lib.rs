use bevy::{prelude::*, window::WindowResolution};
use bevy_web_video::{
    events, EventSender, EventWithAssetId, VideoElement, VideoElementCreated, VideoElementRegistry, WebVideo, WebVideoError, WebVideoPlugin
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
) {
    let image = images.reserve_handle();
    let video_element = video_elements.add(VideoElement::new(&image));

    commands
        .spawn(WebVideo::new(video_element))
        .observe(video_created_observer);

    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 3.0)));
}

fn video_created_observer(
    trigger: Trigger<VideoElementCreated>,
    mut commands: Commands,
    loadedmetadata_event_sender: Res<EventSender<events::LoadedMetadata>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) -> Result<()> {
    if let Some(element) = registry.element(trigger.asset_id()) {
        commands
            .entity(trigger.target())
            .observe(video_loaded_observer);

        loadedmetadata_event_sender.enable_element_event_observers(
            trigger.asset_id(),
            &element,
            &mut registry,
            trigger.target(),
        );

        element.set_cross_origin(Some("anonymous"));
        element.set_src(
            "https://thepaciellogroup.github.io/AT-browser-tests/video/ElephantsDream.mp4",
        );
        element.set_loop(true);

         let track = registry.document().create_element("track").inspect_err(|e| warn!("{e:?}"))
        .unwrap_throw()
        .dyn_into::<web_sys::HtmlTrackElement>()
        .inspect_err(|e| warn!("{e:?}"))
        .expect_throw("web_sys::HtmlTrackElement");
         track.set_kind("subtitles");
         track.set_srclang("en");
         track.set_src("https://thepaciellogroup.github.io/AT-browser-tests/video/subtitles-en.vtt");
         track.set_default(true);
        }

        let _ = element.play().map_err(WebVideoError::from)?;

        Ok(())
    } else {
        Err("missing video".into())
    }
}

fn video_loaded_observer(trigger: Trigger<ListenerEvent<events::LoadedMetadata>>) {}
