[![Crates.io](https://img.shields.io/crates/v/wasm_plugin_guest.svg)](https://crates.io/crates/wasm_plugin_guest)
[![Docs.rs](https://docs.rs/wasm_plugin_guest/badge.svg)](https://docs.rs/wasm_plugin_guest)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

A low-ish level tool for easily writing WASM based plugins to be hosted by
wasm_plugin_host.

The goal of wasm_plugin is to make communicating across the host-plugin
boundary as simple and idiomatic as possible while being unopinionated
about how you actually use the plugin.

Plugins are meant to be run using [wasm_plugin_host](https://crates.io/crates/wasm_plugin_host)

Exporting a function is just a matter of adding an attribute.

```rust
#[wasm_plugin_guest::export_function]
fn hello() -> String {
    "Hello, host!".to_string()
}
```

## API Stability

I am not currently guaranteeing any stability, expect all releases to include breaking changes.
