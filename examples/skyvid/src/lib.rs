use bevy::{prelude::*, window::WindowResolution};
use wasm_bindgen::prelude::*;
mod bluesky;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(700, 700),
                ..default()
            }),
            ..default()
        }),
        bluesky::plugin,
    ));

    app.run();
}
