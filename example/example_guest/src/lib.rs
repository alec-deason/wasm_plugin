fn local_hello() -> String {
    "Hello, Host!".to_string()
}

fn local_echo(message: String) -> String {
    message
}

fn local_favorite_numbers() -> Vec<i32> {
    vec![1, 2, 43]
}

wasm_plugin_guest::export_plugin_function_with_no_input!(hello, local_hello);
wasm_plugin_guest::export_plugin_function_with_input_message!(echo, local_echo);
wasm_plugin_guest::export_plugin_function_with_no_input!(favorite_numbers, local_favorite_numbers);
