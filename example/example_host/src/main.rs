use host::WasmPlugin;

fn main() {
    let mut plugin = WasmPlugin::load("../example_guest/target/wasm32-unknown-unknown/release/example_guest.wasm").unwrap();
    let response: String = plugin.call_function("hello").unwrap();
    println!("The guest says: '{}'", response);

    let message = "Hello, Guest!".to_string();
    let response: String = plugin.call_function_with_message("echo", &message).unwrap();
    println!("I said: '{}'. The guest said, '{}' back. Weird", message, response);
}
