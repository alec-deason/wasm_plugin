#![doc(html_root_url = "https://docs.rs/wasm_plugin/guest/0.1.0")]
#![deny(missing_docs)]

//! A low-ish level tool for easily writing WASM based plugins to be hosted by
//! wasm_plugin_host.
//!
//! The goal of wasm_plugin is to make communicating across the host-plugin
//! boundary as simple and idiomatic as possible while being unopinionated
//! about how you actually use the plugin.
//!

#[no_mangle]
static mut MESSAGE_BUFFER: [u8; 1024 * 10] = [0; 1024 * 10];

#[cfg(feature = "inject_getrandom")]
mod getrandom_shim {
    use getrandom::register_custom_getrandom;

    use getrandom::Error;

    extern "C" {
        fn __getrandom(ptr: i32, len: i32);
    }

    #[allow(clippy::unnecessary_wraps)]
    fn external_getrandom(buf: &mut [u8]) -> Result<(), Error> {
        let len = buf.len();
        let ptr = buf.as_ptr();
        unsafe {
            __getrandom(ptr as i32, len as i32);
        }
        Ok(())
    }
    register_custom_getrandom!(external_getrandom);
}

/// Read a message from the buffer used to communicate with the host. You should
/// never need to call this directly.
pub fn read_message<T: serde::de::DeserializeOwned>() -> T {
    let buf = unsafe { &mut MESSAGE_BUFFER };
    bincode::deserialize(buf).unwrap()
}

/// Write a message to the buffer used to communicate with the host. You should
/// never need to call this directly.
pub fn write_message<U>(message: &U) -> i32
where
    U: serde::Serialize,
{
    let buf = unsafe { &mut MESSAGE_BUFFER };
    let message: Vec<u8> = bincode::serialize(message).unwrap();
    let len = message.len();
    buf[0..len].copy_from_slice(&message);
    len as i32
}

/// Export a function with no arguments to the plugin host
#[macro_export]
macro_rules! export_plugin_function_with_no_input {
    ($name:ident, $function:path) => {
        const _: () = {
            #[no_mangle]
            pub extern "C" fn $name() -> i32 {
                wasm_plugin_guest::write_message(&$function())
            }
        };
    };
}

/// Export a function with a single argument to the plugin host
#[macro_export]
macro_rules! export_plugin_function_with_input_message {
    ($name:ident, $function:path) => {
        const _: () = {
            #[no_mangle]
            pub extern "C" fn $name() -> i32 {
                let message = wasm_plugin_guest::read_message();

                wasm_plugin_guest::write_message(&$function(message))
            }
        };
    };
}
