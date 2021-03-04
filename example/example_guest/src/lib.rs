#[wasm_plugin_guest::export_function]
fn hello() -> String {
    "Hello, Host!".to_string()
}

#[wasm_plugin_guest::export_function]
fn echo(message: String, message2: String) -> String {
    format!("{} {}", message, message2)
}

#[wasm_plugin_guest::export_function]
fn favorite_numbers() -> Vec<i32> {
    vec![1, 2, 43]
}
