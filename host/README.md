<img src="https://docs.rs/wasm_plugin_host/badge.svg" alt="wasm_plugin_host">


A low-ish level tool for easily hosting WASM based plugins.


The goal of wasm_plugin is to make communicating across the host-plugin
boundary as simple and idiomatic as possible while being unopinionated
 about how you actually use the plugin.
 
 
Loading a plugin is as simple as reading the .wasm file off disk.

```rust
let mut plugin = WasmPlugin::load("path/to/plugin.wasm")?;
```

Calling functions exported by the plugin takes one of two forms. Either
 the function takes no arguments and returns a single serde deserializable
value:

```rust
let response: ResultType = plugin.call_function("function_name")?;
```

Or it takes a single serializable argument and returns a single result:

```rust
let message = Message::default();
let response: ResultType = plugin.call_function_with_argument("function_name", &message)?;
```
