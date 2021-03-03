<img src="https://docs.rs/wasm_plugin_guest/badge.svg" alt="wasm_plugin_guest">

A low-ish level tool for easily writing WASM based plugins to be hosted by
wasm_plugin_host.

The goal of wasm_plugin is to make communicating across the host-plugin
boundary as simple and idiomatic as possible while being unopinionated
about how you actually use the plugin.

Exporting a function is just a matter of wrapping it in a macro:

```rust
fn local_hello() -> String {
    "Hello, host!".to_string()
}
wasm_plugin_guest::export_plugin_function_with_no_input(hello, local_hello);
```
