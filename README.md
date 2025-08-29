# bevy_web_video

Streaming video on Bevy textures for webgpu and webgl2 targets.

See [examples/cubes/src/lib.rs](examples/cubes/src/lib.rs) for example use.
[Live demo](https://rectalogic.com/bevy_web_video/) of the example.

To run the demo locally:
```sh-session
$ cargo install wasm-opt wasm-bindgen-cli
$ cargo build --profile debug --target wasm32-unknown-unknown --example cubes
$ wasm-bindgen --out-dir examples/wasm/target --out-name wasm_example --target web target/wasm32-unknown-unknown/debug/examples/{example}.wasm
$ wasm-opt -Oz --output examples/wasm/target/wasm_example_bg.wasm.optimized examples/wasm/target/wasm_example_bg.wasm
# rename wasm.optimized
$ python3 -m http.server -d examples/cubes  # now open http://localhost:8000/
```
XXX see https://github.com/bevyengine/bevy/blob/6608d9815da46e1c79ec3887568a068e63065f49/tools/build-wasm-example/src/main.rs
