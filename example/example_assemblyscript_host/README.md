This example demonstrates hosting a non-rust plugin using json serialization rather than bincode.

To build the assemblyscript plugin:
```
cd assemblyscript_plugin
npm run asbuild
```

Then run the host with:
```
cargo run
```

There is currently no non-rust equivalent to the wasm_plugin_guest crate to make writing plugins in assemblyscript, or any other language, ergonomic. Those may be added at some point. This example demonstrates how to manually implement the wasm_plugin calling convention and serialization in assemblyscript and the  it should be reasonably straightforward to do the same thing in any other WASM targeting language.
