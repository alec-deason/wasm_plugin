#![doc(html_root_url = "https://docs.rs/wasm_plugin_host/0.1.4")]
#![deny(missing_docs)]

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
//!
//! Currently serialization uses either bincode or json, selected by feature:
//! `serialize_bincode`: Uses serde and bincode. It is selected by default.
//! `serialize_json`: Uses serde and serde_json.
//! `serialize_nanoserde_json': Uses nanoserde.
//!
//! Bincode is likely the best choice if all plugins the system uses will be
//! written in Rust. Json is useful if a mix or languages will be used.
//!
//! ## Limitations
//!
//! There is no reflection so you must know up front which functions
//! a plugin exports and their signatures.

use std::path::Path;

use wasmer::{
    Exports, Function, Global, Instance, LazyInit, Memory, MemoryView, Module, Store, Value,
    WasmerEnv,
};
pub use wasmer::{Extern, HostFunction};

#[allow(missing_docs)]
pub mod errors;
#[allow(missing_docs)]
pub mod serialization;
use serialization::{Deserializable, Serializable};

/// Constructs a WasmPlugin
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
        env.insert(
            "abort",
            Function::new_native(&store, |_: i32, _: i32, _: i32, _: i32| {}),
        );
        #[cfg(feature = "inject_getrandom")]
        {
            env.insert(
                "__getrandom",
                Function::new_native_with_env(&store, Env::default(), getrandom_shim),
            );
        }

        Ok(Self { module, store, env })
    }

    fn import(mut self, name: impl ToString, value: impl Into<Extern>) -> Self {
        let name = format!("wasm_plugin_imported__{}", name.to_string());
        self.env.insert(name, value);
        self
    }

    /// Import a function defined in the host into the guest. The function's
    /// arguments and return type must all be serializable.
    pub fn import_function<Args, F: ImportableFn<Args> + Send + 'static>(
        self,
        name: impl ToString,
        value: F,
    ) -> Self {
        #[derive(WasmerEnv, Clone, Default)]
        struct Env {
            #[wasmer(export(name = "MESSAGE_BUFFER"))]
            buffer: LazyInit<Global>,
            #[wasmer(export)]
            memory: LazyInit<Memory>,
        }

        let env = Env::default();
        if F::has_arg() {
            let f = if F::has_return() {
                let wrapped = move |env: &Env, len: i32| -> i32 {
                    let buffer = MessageBuffer {
                        buffer: unsafe { env.buffer.get_unchecked() }.get(),
                        memory: unsafe { env.memory.get_unchecked() },
                    };
                    value.call_with_input(buffer, len as usize).unwrap() as i32
                };
                Function::new_native_with_env(&self.store, env, wrapped)
            } else {
                let wrapped = move |env: &Env, len: i32| {
                    let buffer = MessageBuffer {
                        buffer: unsafe { env.buffer.get_unchecked() }.get(),
                        memory: unsafe { env.memory.get_unchecked() },
                    };
                    value.call_with_input(buffer, len as usize).unwrap();
                };
                Function::new_native_with_env(&self.store, env, wrapped)
            };
            self.import(name, f)
        } else {
            let f = if F::has_return() {
                let wrapped = move |env: &Env| -> i32 {
                    let buffer = MessageBuffer {
                        buffer: unsafe { env.buffer.get_unchecked() }.get(),
                        memory: unsafe { env.memory.get_unchecked() },
                    };
                    value.call_without_input(buffer).unwrap() as i32
                };
                Function::new_native_with_env(&self.store, env, wrapped)
            } else {
                let wrapped = move |env: &Env| {
                    let buffer = MessageBuffer {
                        buffer: unsafe { env.buffer.get_unchecked() }.get(),
                        memory: unsafe { env.memory.get_unchecked() },
                    };
                    value.call_without_input(buffer).unwrap();
                };
                Function::new_native_with_env(&self.store, env, wrapped)
            };
            self.import(name, f)
        }
    }

    /// Finalize the builder and create the WasmPlugin ready for use.
    pub fn finish(self) -> errors::Result<WasmPlugin> {
        let mut import_object = wasmer::ImportObject::new();
        import_object.register("env", self.env);
        Ok(WasmPlugin {
            instance: Instance::new(&self.module, &import_object)?,
        })
    }
}

/// A marker trait for Fn types who's arguments and return type can be
/// serialized and are thus safe to import into a plugin;
pub trait ImportableFn<ArgList> {
    #[doc(hidden)]
    fn has_arg() -> bool;
    #[doc(hidden)]
    fn has_return() -> bool;
    #[doc(hidden)]
    fn call_with_input(&self, message_buffer: MessageBuffer, len: usize) -> errors::Result<usize>;
    #[doc(hidden)]
    fn call_without_input(&self, message_buffer: MessageBuffer) -> errors::Result<usize>;
}

impl<F, Args, ReturnType> ImportableFn<Args> for F
where
    F: Fn(Args) -> ReturnType,
    Args: Deserializable,
    ReturnType: Serializable,
{
    fn has_arg() -> bool {
        true
    }
    fn has_return() -> bool {
        std::mem::size_of::<ReturnType>() > 0
    }
    fn call_with_input(&self, message_buffer: MessageBuffer, len: usize) -> errors::Result<usize> {
        let message = message_buffer.read_message(len);
        let result = self(Args::deserialize(&message)?);
        if std::mem::size_of::<ReturnType>() > 0 {
            // No need to write anything for ZSTs
            let message = result.serialize()?;
            Ok(message_buffer.write_message(&message))
        } else {
            Ok(0)
        }
    }

    fn call_without_input(&self, _message_buffer: MessageBuffer) -> errors::Result<usize> {
        unimplemented!("Requires argument")
    }
}

#[doc(hidden)]
pub enum NoArgs {}

impl<F, ReturnType> ImportableFn<NoArgs> for F
where
    F: Fn() -> ReturnType,
    ReturnType: Serializable,
{
    fn has_arg() -> bool {
        false
    }
    fn has_return() -> bool {
        std::mem::size_of::<ReturnType>() > 0
    }
    fn call_with_input(
        &self,
        _message_buffer: MessageBuffer,
        _len: usize,
    ) -> errors::Result<usize> {
        unimplemented!("Must not supply argument")
    }

    fn call_without_input(&self, message_buffer: MessageBuffer) -> errors::Result<usize> {
        let result = self();
        if std::mem::size_of::<ReturnType>() > 0 {
            // No need to write anything for ZSTs
            let message = result.serialize()?;
            Ok(message_buffer.write_message(&message))
        } else {
            Ok(0)
        }
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

#[doc(hidden)]
pub struct MessageBuffer<'a> {
    buffer: Value,
    memory: &'a Memory,
}

impl<'a> MessageBuffer<'a> {
    fn write_message(&self, message: &[u8]) -> usize {
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
        len as usize
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
            buff.copy_from_slice(&data[memory_idx as usize..memory_idx as usize + len]);
        }
        buff
    }
}

impl WasmPlugin {
    fn message_buffer(&self) -> errors::Result<MessageBuffer> {
        Ok(MessageBuffer {
            memory: self.instance.exports.get_memory("memory")?,
            buffer: self.instance.exports.get::<Global>("MESSAGE_BUFFER")?.get(),
        })
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
        Args: Serializable,
        ReturnType: Deserializable,
    {
        let message = args.serialize()?;
        self.message_buffer()?.write_message(&message);

        self.call_function(fn_name)
    }

    fn call_function_raw(&mut self, fn_name: &str) -> errors::Result<Vec<u8>> {
        let f = self
            .instance
            .exports
            .get_function(&format!("wasm_plugin_exported__{}", fn_name))
            .unwrap_or_else(|_| panic!("Unable to find function {}", fn_name));

        let result_len = f.native::<(), i32>()?.call()?;

        Ok(self.message_buffer()?.read_message(result_len as usize))
    }

    /// Call a function exported by the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    pub fn call_function<ReturnType>(&mut self, fn_name: &str) -> errors::Result<ReturnType>
    where
        ReturnType: Deserializable,
    {
        let buff = self.call_function_raw(fn_name)?;
        ReturnType::deserialize(&buff)
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
