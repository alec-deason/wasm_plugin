#![doc(html_root_url = "https://docs.rs/wasm_plugin_guest/0.1.4")]
#![deny(missing_docs)]

//! A low-ish level tool for easily writing WASM based plugins to be hosted by
//! wasm_plugin_host.
//!
//! The goal of wasm_plugin is to make communicating across the host-plugin
//! boundary as simple and idiomatic as possible while being unopinionated
//! about how you actually use the plugin.
//!
//! This crate currently supports serialization either using bincode or json
//! selected by feature:
//! `serialize_bincode`: Uses serde and bincode. It is selected by default.
//! `serialize_json`: Uses serde and serde_json.
//! `serialize_nanoserde_json': Uses nanoserde.
//!
//! Bincode is likely the best choice if all plugins the system uses will be
//! written in Rust. Json is useful if a mix or languages will be used.
//!
//! Plugins are meant to be run using [wasm_plugin_host](https://crates.io/crates/wasm_plugin_host)

use std::mem::ManuallyDrop;

mod serialization;
pub use wasm_plugin_guest_derive::{export_function, import_functions};

bitfield::bitfield! {
    #[doc(hidden)]
    pub struct FatPointer(u64);
    u32;
    #[doc(hidden)]
    pub ptr, set_ptr: 31, 0;
    #[doc(hidden)]
    pub len, set_len: 63, 32;
}

/// Read a message from a buffer created with `allocate_message_buffer`. You should
/// never need to call this directly.
pub fn read_message<T: serialization::Deserializable>(ptr: usize, len: usize) -> T {
    let buf = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    T::deserialize(buf)
}

/// Write a message to the buffer used to communicate with the host. You should
/// never need to call this directly.
pub fn write_message<U>(message: &U) -> (usize, usize)
where
    U: serialization::Serializable,
{
    let message: Vec<u8> = message.serialize();
    let local_len = message.len();
    (
        ManuallyDrop::new(message).as_mut_ptr() as *const usize as usize,
        local_len,
    )
}

#[cfg(feature = "inject_getrandom")]
mod getrandom_shim {
    use getrandom::register_custom_getrandom;

    use getrandom::Error;

    extern "C" {
        fn __getrandom(ptr: u32, len: u32);
    }

    #[allow(clippy::unnecessary_wraps)]
    fn external_getrandom(buf: &mut [u8]) -> Result<(), Error> {
        let len = buf.len();
        let ptr = buf.as_ptr();
        unsafe {
            __getrandom(ptr as u32, len as u32);
        }
        Ok(())
    }
    register_custom_getrandom!(external_getrandom);
}

/// Allocate a buffer suitable for writing messages to and return it's address.
#[no_mangle]
pub extern "C" fn allocate_message_buffer(len: u32) -> u32 {
    let mut buffer: ManuallyDrop<Vec<u8>> = ManuallyDrop::new(Vec::with_capacity(len as usize));
    buffer.as_mut_ptr() as *const u32 as u32
}

/// Frees a previously allocated buffer.
#[no_mangle]
pub extern "C" fn free_message_buffer(ptr: u32, len: u32) {
    unsafe { drop(Vec::from_raw_parts(ptr as *mut u8, 0, len as usize)) }
}
