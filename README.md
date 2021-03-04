A low-ish level tool for easily writing and hosting WASM based plugins.

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

Exporting a function from a plugin is just a matter of wrapping it in a macro:

```rust
fn local_hello() -> String {
    "Hello, host!".to_string()
}
wasm_plugin_guest::export_plugin_function_with_no_input(hello, local_hello);
```

## API Stability

I am not currently guaranteeing any stability, expect all releases to include breaking changes.
