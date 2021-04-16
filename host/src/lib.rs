#![doc(html_root_url = "https://docs.rs/wasm_plugin_host/0.1.2")]
//#![deny(missing_docs)]

//! A low-ish level tool for easily hosting WASM based plugins.
//!
//! The goal of wasm_plugin is to make communicating across the host-plugin
//! boundary as simple and idiomatic as possible while being unopinionated
//!  about how you actually use the plugin.
//!
//! Plugins should be written using [wasm_plugin_guest](https://crates.io/crates/wasm_plugin_guest)
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

use std::{
    path::Path,
};

use wasmer::{
    internals::{WithEnv, WithoutEnv},
    imports, Function, Global, Instance, LazyInit, Memory, MemoryView, Module, Store, Value,
    WasmerEnv, Exports, WasmTypeList,
};
pub use wasmer::{Extern, HostFunction};

#[allow(missing_docs)]
pub mod errors;

pub struct WasmPluginBuilder {
    module: Module,
    store: Store,
    env: Exports,
}
impl WasmPluginBuilder {
    /// Load a plugin off disk and prepare it for use.
    pub fn from_file(path: impl AsRef<Path>) -> errors::Result<Self> {
        let source = std::fs::read(path)?;
        Self::from_source(&source)
    }

    /// Load a plugin from WASM source and prepare it for use.
    pub fn from_source(source: &[u8]) -> errors::Result<Self> {
        let store = Store::default();
        let module = Module::new(&store, source)?;
        let mut env = wasmer::Exports::new();
        env.insert("abort", Function::new_native(&store, |_: i32, _:i32, _:i32, _:i32| {}));
        #[cfg(feature = "inject_getrandom")]
        {
            env.insert("__getrandom", Function::new_native_with_env(&store, Env::default(), getrandom_shim));
        }

        Ok(Self {
            module,
            store,
            env,
        })
    }

    fn import(mut self, name: impl Into<String>, value: impl Into<Extern>) -> Self {
        self.env.insert(name, value);
        self
    }

    pub fn import_function<F, Args, ReturnType>(mut self, name: impl Into<String>, value: F) -> Self
    where
        F: Fn(Args) -> ReturnType + Send + 'static,
        ReturnType: serde::Serialize,
        Args: serde::de::DeserializeOwned + Clone,
    {
        #[derive(WasmerEnv, Clone, Default)]
        struct Env {
            #[wasmer(export(name = "MESSAGE_BUFFER"))]
            buffer: LazyInit<Global>,
            #[wasmer(export)]
            memory: LazyInit<Memory>,
        }

        let env = Env::default();
        let wrapped = move |env: &Env, len: i32| {
            let buffer = MessageBuffer {
                buffer: unsafe { env.buffer.get_unchecked() }.get(),
                memory: unsafe { env.memory.get_unchecked() },
            };
            let message = buffer.read_message(len as usize);
            let result = value(bincode::deserialize(&message).unwrap());
            let message = bincode::serialize(&result).unwrap();
            buffer.write_message(&message)
        };
        let f = Function::new_native_with_env(&self.store, env, wrapped);
        self.import(name, f)
    }

    pub fn finish(self) -> errors::Result<WasmPlugin> {
        let mut import_object = wasmer::ImportObject::new();
        import_object.register("env", self.env);
        Ok(WasmPlugin {
            instance: Instance::new(&self.module, &import_object)?,
        })
    }
}

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

struct MessageBuffer<'a> {
    buffer: Value,
    memory: &'a Memory
}

impl<'a> MessageBuffer<'a> {
    fn write_message(&self, message: &[u8]) {
        let memory_idx = if let Value::I32(memory_idx) = self.buffer {
            memory_idx
        } else {
            panic!();
        };
        let len = message.len() as i32;

        unsafe {
            let data = self.memory.data_unchecked_mut();
            data[memory_idx as usize..memory_idx as usize + len as usize].copy_from_slice(&message);
        }
    }

    fn read_message(&self, len: usize) -> Vec<u8> {
        let memory_idx = if let Value::I32(memory_idx) = self.buffer {
            memory_idx
        } else {
            panic!();
        };
        let mut buff: Vec<u8> = vec![0; len];
        unsafe {
            let data = self.memory.data_unchecked();
            buff.copy_from_slice(
                &data[memory_idx as usize..memory_idx as usize + len],
            );
        }
        buff
    }

}

impl WasmPlugin {
    fn message_buffer(&self) -> errors::Result<MessageBuffer> {
        Ok(MessageBuffer {
            memory: self.instance.exports.get_memory("memory").unwrap(),
            buffer: self
                .instance
                .exports
                .get::<Global>("MESSAGE_BUFFER")
                .unwrap()
                .get()
        })
    }


    /// Call a function exported by the plugin with a single argument
    /// which will be serialized and sent to the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    #[cfg(feature = "serialize_bincode")]
    pub fn call_function_with_argument<ReturnType, Args>(
        &mut self,
        fn_name: &str,
        args: &Args,
    ) -> errors::Result<ReturnType>
    where
        Args: serde::Serialize,
        ReturnType: serde::de::DeserializeOwned + Clone,
    {
        let message =
            bincode::serialize(args).map_err(|_| errors::WasmPluginError::SerializationError)?;
        self.message_buffer()?.write_message(&message);

        self.call_function(fn_name)
    }

    /// Call a function exported by the plugin with a single argument
    /// which will be serialized and sent to the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    #[cfg(feature = "serialize_nanoserde_json")]
    pub fn call_function_with_argument<ReturnType, Args>(
        &mut self,
        fn_name: &str,
        args: &Args,
    ) -> errors::Result<ReturnType>
    where
        Args: nanoserde::SerJson,
        ReturnType: nanoserde::DeJson,
    {
        let message =
            nanoserde::SerJson::serialize_json(args);
        self.write_message(message.as_bytes());

        self.call_function(fn_name)
    }

    fn call_function_raw(&mut self, fn_name: &str) -> errors::Result<Vec<u8>> {
        let f = self
            .instance
            .exports
            .get_function(&format!("wasm_plugin_exported__{}", fn_name))
            .expect(&format!("Unable to find function {}", fn_name));


        let result_len = f.native::<(), i32>()?.call()?;

        Ok(self.message_buffer()?.read_message(result_len as usize))
    }

    /// Call a function exported by the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    #[cfg(feature = "serialize_bincode")]
    pub fn call_function<ReturnType>(&mut self, fn_name: &str) -> errors::Result<ReturnType>
    where
        ReturnType: serde::de::DeserializeOwned + Clone,
    {
        let buff = self.call_function_raw(fn_name)?;
        Ok(bincode::deserialize(&buff)
            .map_err(|_| errors::WasmPluginError::DeserializationError)?)
    }

    /// Call a function exported by the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    #[cfg(feature = "serialize_nanoserde_json")]
    pub fn call_function<ReturnType>(&mut self, fn_name: &str) -> errors::Result<ReturnType>
    where
        ReturnType: nanoserde::DeJson,
    {
        let buff = self.call_function_raw(fn_name)?;

        Ok(nanoserde::DeJson::deserialize_json(&String::from_utf8(buff)?)
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
