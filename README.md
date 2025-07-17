# bevy_web_video

Streaming video on Bevy textures for webgpu and webgl2 targets.

See [examples/cubes/src/lib.rs](examples/cubes/src/lib.rs) for example use.
[Live demo](https://rectalogic.com/bevy_web_video/) of the example.

To run the demo locally:
```sh-session
$ cargo install wasm-pack
$ wasm-pack build --target web examples/cubes --features webgpu
$ python -m http.server -d examples/cubes  # now open http://localhost:8000/
```
