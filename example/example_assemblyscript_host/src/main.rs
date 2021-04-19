use wasm_plugin_host::WasmPluginBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = WasmPluginBuilder::from_file(
        "./assemblyscript_plugin/build/optimized.wasm",
    )?
    .import_function("please_capitalize_this", |mut s: String| {
        s.make_ascii_uppercase();
        s
    })
    .finish()?;
    let response: String = plugin.call_function("hello")?;
    println!("The guest says: '{}'", response);

    let message = "Hello, Guest!".to_string();
    let response: String = plugin.call_function_with_argument("echo", &message)?;
    println!(
        "I said: '{}'. The guest said, '{}' back. Weird",
        message, response
    );

    // Any type that can be serialized works
    let response: Vec<i32> = plugin.call_function("favorite_numbers")?;
    println!(
        "The guest's favorite integers (less that 2**32) are: '{:?}'",
        response
    );

    Ok(())
}
