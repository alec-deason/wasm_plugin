use wasm_plugin_host::WasmPluginBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugin = WasmPluginBuilder::from_file(
        "../example_guest/target/wasm32-unknown-unknown/release/example_guest.wasm",
    )?.import_function("the_hosts_favorite_numbers", |arg: ()| {
        println!("poop");
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
