#![doc(html_root_url = "https://docs.rs/wasm_plugin/host/0.1.0")]
#![deny(missing_docs)]

//! A low-ish level tool for easily hosting WASM based plugins.
//!
//! The goal of wasm_plugin is to make communicating across the host-plugin
//! boundary as simple and idiomatic as possible while being unopinionated
//!  about how you actually use the plugin.
//!
//! Loading a plugin is as simple as reading the .wasm file off disk.
//!
//! ```rust
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let mut plugin = WasmPlugin::load("path/to/plugin.wasm")?;
//! #
//! #     Ok(())
//! # }
//! ```
//!
//! Calling functions exported by the plugin takes one of two forms. Either
//!  the function takes no arguments and returns a single serde deserializable
//! value:
//!
//! ```rust
//! # #[derive(Deserialize)]
//! # struct ResultType;
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let mut plugin = WasmPlugin::load("path/to/plugin.wasm")?;
//! let response: ResultType = plugin.call_function("function_name")?;
//! #
//! #     Ok(())
//! # }
//! ```
//! Or it takes a single serializable argument and returns a single result:
//! ```rust
//! # #[derive(Deserialize)]
//! # struct ResultType;
//! # #[derive(Serialize, Default)]
//! # struct Message;
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let mut plugin = WasmPlugin::load("path/to/plugin.wasm")?;
//! let message = Message::default();
//! let response: ResultType = plugin.call_function_with_argument("function_name", &message)?;
//! #
//! #     Ok(())
//! # }
//! ```
//! If the `inject_getrandom` feature is selected then the host's getrandom
//! will be injected into the plugin which allows `rand` to be used in the
//! plugin. `inject_getrandom` is selected by default.
//! ## Limitations
//!
//! Currently serialization is done using bincode which limits plugins to being
//! written in rust. This may change in the future.
//!
//! There is no reflection so you must know up front which functions
//! a plugin exports and their signatures.

use std::path::Path;

use wasmer::{
    imports, Function, Global, Instance, LazyInit, Memory, MemoryView, Module, Store, Value,
    WasmerEnv,
};

#[allow(missing_docs)]
pub mod errors;

/// A loaded plugin
#[derive(Clone, Debug)]
pub struct WasmPlugin {
    instance: Instance,
}

#[derive(WasmerEnv, Clone, Default, Debug)]
struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

impl WasmPlugin {
    /// Load a plugin from WASM source and prepare it for use.
    pub fn new(source: &[u8]) -> errors::Result<Self> {
        let store = Store::default();
        let import_object;
        #[cfg(feature = "inject_getrandom")]
        {
            import_object = imports! {
                "env" => { "__getrandom" => Function::new_native_with_env(&store, Env::default(), getrandom_shim), },
            };
        }
        #[cfg(not(feature = "inject_getrandom"))]
        {
            import_object = imports! {
                "env" => {},
            };
        }
        let module = Module::new(&store, source)?;

        let instance = Instance::new(&module, &import_object)?;
        Ok(Self { instance })
    }

    /// Load a plugin off disk and prepare it for use.
    pub fn load(path: impl AsRef<Path>) -> errors::Result<Self> {
        let source = std::fs::read(path)?;
        WasmPlugin::new(&source)
    }

    /// Call a function exported by the plugin with a single argument
    /// which will be serialized and sent to the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    pub fn call_function_with_argument<ReturnType, Args>(
        &mut self,
        fn_name: &str,
        args: &Args,
    ) -> errors::Result<ReturnType>
    where
        Args: serde::Serialize,
        ReturnType: serde::de::DeserializeOwned + Clone,
    {
        let buffer = self
            .instance
            .exports
            .get::<Global>("MESSAGE_BUFFER")
            .unwrap()
            .get();
        let memory_idx = if let Value::I32(memory_idx) = buffer {
            memory_idx
        } else {
            panic!();
        };
        let memory = self.instance.exports.get_memory("memory").unwrap();
        // TODO: I don't really want to expose bincode in the public API but there may be cases where this obscures useful information about the actual error.
        let message =
            bincode::serialize(args).map_err(|_| errors::WasmPluginError::SerializationError)?;
        let len = message.len() as i32;

        unsafe {
            let data = memory.data_unchecked_mut();
            data[memory_idx as usize..memory_idx as usize + len as usize].copy_from_slice(&message);
        }

        self.call_function(fn_name)
    }

    /// Call a function exported by the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    pub fn call_function<ReturnType>(&mut self, fn_name: &str) -> errors::Result<ReturnType>
    where
        ReturnType: serde::de::DeserializeOwned + Clone,
    {
        let f = self.instance.exports.get_function(fn_name).unwrap();

        let buffer = self
            .instance
            .exports
            .get::<Global>("MESSAGE_BUFFER")
            .unwrap()
            .get();
        let memory_idx = if let Value::I32(memory_idx) = buffer {
            memory_idx
        } else {
            panic!();
        };
        let memory = self.instance.exports.get_memory("memory").unwrap();

        let result_len = f.native::<(), i32>()?.call()?;

        let mut buff: Vec<u8> = vec![0; result_len as usize];
        unsafe {
            let data = memory.data_unchecked();
            buff.copy_from_slice(
                &data[memory_idx as usize..memory_idx as usize + result_len as usize],
            );
        }
        Ok(bincode::deserialize(&buff)
            .map_err(|_| errors::WasmPluginError::DeserializationError)?)
    }
}

#[cfg(feature = "inject_getrandom")]
fn getrandom_shim(env: &Env, ptr: i32, len: i32) {
    if let Some(memory) = env.memory_ref() {
        let view: MemoryView<u8> = memory.view();
        let mut buff: Vec<u8> = vec![0; len as usize];
        getrandom::getrandom(&mut buff).unwrap();
        for (dst, src) in view[ptr as usize..ptr as usize + len as usize]
            .iter()
            .zip(buff)
        {
            dst.set(src);
        }
    }
}
