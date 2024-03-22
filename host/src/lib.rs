#![doc(html_root_url = "https://docs.rs/wasm_plugin_host/0.1.7")]
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
//! let mut plugin = WasmPluginBuilder::from_file("path/to/plugin.wasm")?.finish()?;
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
//! # let mut plugin = WasmPluginBuilder::from_file("path/to/plugin.wasm")?.finish()?;
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
//! # let mut plugin = WasmPluginBuilder::from_file("path/to/plugin.wasm")?.finish()?;
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
//! written in Rust. Json is useful if a mix of languages will be used.
//!
//! ## Limitations
//!
//! There is no reflection so you must know up front which functions
//! a plugin exports and their signatures.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use wasmer::FunctionEnvMut;
use wasmer::{
    AsStoreMut, AsStoreRef, Exports, Function, FunctionEnv, Instance, Memory, MemoryView, Module,
    Store, TypedFunction,
};
pub use wasmer::{Extern, HostFunction};

#[allow(missing_docs)]
pub mod errors;
#[allow(missing_docs)]
pub mod serialization;
use bitfield::bitfield;
use serialization::{Deserializable, Serializable};

bitfield! {
    #[doc(hidden)]
    pub struct FatPointer(u64);
    impl Debug;
    u32;
    ptr, set_ptr: 31, 0;
    len, set_len: 63, 32;
}

struct Env<C>
where
    C: Send + Sync + Clone + 'static,
{
    memory: Option<Memory>,
    allocator: Option<TypedFunction<u32, u32>>,
    garbage: Arc<Mutex<Vec<FatPointer>>>,
    ctx: C,
}

impl<C: Send + Sync + Clone + 'static> Env<C> {
    fn new(garbage: Arc<Mutex<Vec<FatPointer>>>, ctx: C) -> Self {
        Self {
            allocator: None,
            memory: None,
            garbage,
            ctx,
        }
    }

    fn message_buffer(&self) -> MessageBuffer {
        unsafe {
            MessageBuffer {
                allocator: OwnedOrRef::Ref(self.allocator.as_ref().unwrap_unchecked()),
                memory: self.memory.as_ref().unwrap_unchecked(),
                garbage: vec![],
            }
        }
    }

    fn memory_ref(&self) -> &Option<Memory> {
        &self.memory
    }
}

/// Used in [`WasmPluginBuilder`] to store a `Vec` of `FunctionEnv<Env<C>>`s with different `C`s
trait EnvExport {
    fn update_exports(&mut self, exports: &Exports, store: &mut Store);
}

impl<C: Send + Sync + Clone + 'static> EnvExport for FunctionEnv<Env<C>> {
    fn update_exports(&mut self, exports: &Exports, store: &mut Store) {
        let mut fenvm = self.clone().into_mut(store);
        let (data, store) = fenvm.data_and_store_mut();
        data.allocator = Some(
            exports
                .get_typed_function(&store, "allocate_message_buffer")
                .unwrap(),
        );
        data.memory = Some(exports.get_memory("memory").unwrap().clone());
    }
}

/// Constructs a WasmPlugin
pub struct WasmPluginBuilder {
    module: Module,
    store: Store,
    env: Exports,
    // TODO: Can we do this without the lock?
    garbage: Arc<Mutex<Vec<FatPointer>>>,
    // need to save these to update the allocator and memory exports in `Self::finish`
    envs: Vec<Box<dyn EnvExport>>,
}
impl WasmPluginBuilder {
    /// Load a plugin off disk and prepare it for use.
    pub fn from_file(path: impl AsRef<Path>) -> errors::Result<Self> {
        let source = std::fs::read(path)?;
        Self::from_source(&source)
    }

    /// Load a plugin from WASM source and prepare it for use.
    pub fn from_source(source: &[u8]) -> errors::Result<Self> {
        let mut store = Store::default();
        let module = Module::new(&store, source)?;
        let mut env = wasmer::Exports::new();
        let garbage: Arc<Mutex<Vec<FatPointer>>> = Default::default();
        env.insert(
            "abort",
            Function::new_typed(&mut store, |_: u32, _: u32, _: i32, _: i32| {}),
        );

        let mut envs = vec![];

        #[cfg(feature = "inject_getrandom")]
        {
            let menv = Env::new(garbage.clone(), ());
            let fenv = FunctionEnv::new(&mut store, menv);
            env.insert(
                "__getrandom",
                Function::new_typed_with_env(&mut store, &fenv, getrandom_shim),
            );
            envs.push(Box::new(fenv) as _);
        }

        Ok(Self {
            module,
            store,
            env,
            garbage,
            envs,
        })
    }

    fn import(mut self, name: impl ToString, value: impl Into<Extern>) -> Self {
        let name = format!("wasm_plugin_imported__{}", name.to_string());
        self.env.insert(name, value);
        self
    }

    // FIXME: There is a lot of problematic duplication in this code. I need
    // to sit down and come up with a better abstraction.

    /// Import a function defined in the host into the guest. The function's
    /// arguments and return type must all be serializable.
    /// An immutable reference to `ctx` will be passed to the function as it's
    /// first argument each time it's called.
    ///
    /// NOTE: This method exists due to a limitation in the underlying Waswer
    /// engine which currently doesn't support imported closures with
    /// captured context. The Wasamer developers have said they are interested
    /// in removing that limitation and when they do this method will be
    /// removed in favor of `import_function' since context can be more
    /// idiomatically handled with captured values.
    pub fn import_function_with_context<
        Args,
        F: ImportableFnWithContext<C, Args> + Send + Sync + 'static,
        C: Send + Sync + Clone + 'static,
    >(
        mut self,
        name: impl ToString,
        ctx: C,
        value: F,
    ) -> Self {
        let menv = Env::new(self.garbage.clone(), ctx);
        let env = FunctionEnv::new(&mut self.store, menv);

        if F::has_arg() {
            let f = if F::has_return() {
                let wrapped = move |mut env: FunctionEnvMut<Env<C>>, ptr: u32, len: u32| -> u64 {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    let r = value
                        .call_with_input(
                            &mut buffer,
                            ptr as usize,
                            len as usize,
                            &env.ctx,
                            &mut store,
                        )
                        .unwrap()
                        .map(|p| p.0)
                        .unwrap_or(0);
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                    r
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            } else {
                let wrapped = move |mut env: FunctionEnvMut<Env<C>>, ptr: u32, len: u32| {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    value
                        .call_with_input(
                            &mut buffer,
                            ptr as usize,
                            len as usize,
                            &env.ctx,
                            &mut store,
                        )
                        .unwrap();
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            };
            self.envs.push(Box::new(env) as _);
            self.import(name, f)
        } else {
            let f = if F::has_return() {
                let wrapped = move |mut env: FunctionEnvMut<Env<C>>| -> u64 {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    let r = value
                        .call_without_input(&mut buffer, &env.ctx, &mut store)
                        .unwrap()
                        .map(|p| p.0)
                        .unwrap_or(0);
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                    r
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            } else {
                let wrapped = move |mut env: FunctionEnvMut<Env<C>>| {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    value
                        .call_without_input(&mut buffer, &env.ctx, &mut store)
                        .unwrap();
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            };
            self.envs.push(Box::new(env) as _);
            self.import(name, f)
        }
    }

    /// Import a function defined in the host into the guest. The function's
    /// arguments and return type must all be serializable.
    pub fn import_function<Args, F: ImportableFn<Args> + Send + Sync + 'static>(
        mut self,
        name: impl ToString,
        value: F,
    ) -> Self {
        let menv = Env::new(self.garbage.clone(), ());
        let env = FunctionEnv::new(&mut self.store, menv);

        if F::has_arg() {
            let f = if F::has_return() {
                let wrapped = move |mut env: FunctionEnvMut<Env<()>>, ptr: u32, len: u32| -> u64 {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    let r = value
                        .call_with_input(&mut buffer, ptr as usize, len as usize, &mut store)
                        .unwrap()
                        .map(|p| p.0)
                        .unwrap_or(0);
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                    r
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            } else {
                let wrapped = move |mut env: FunctionEnvMut<Env<()>>, ptr: u32, len: u32| {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    value
                        .call_with_input(&mut buffer, ptr as usize, len as usize, &mut store)
                        .unwrap();
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            };
            self.envs.push(Box::new(env) as _);
            self.import(name, f)
        } else {
            let f = if F::has_return() {
                let wrapped = move |mut env: FunctionEnvMut<Env<()>>| -> u64 {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    let r = value
                        .call_without_input(&mut buffer, &mut store)
                        .unwrap()
                        .map(|p| p.0)
                        .unwrap_or(0);
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                    r
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            } else {
                let wrapped = move |mut env: FunctionEnvMut<Env<()>>| {
                    let (env, mut store) = env.data_and_store_mut();
                    let mut buffer = env.message_buffer();
                    value.call_without_input(&mut buffer, &mut store).unwrap();
                    env.garbage.lock().unwrap().extend(buffer.garbage.drain(..));
                };
                Function::new_typed_with_env(&mut self.store, &env, wrapped)
            };
            self.envs.push(Box::new(env) as _);
            self.import(name, f)
        }
    }

    /// Finalize the builder and create the WasmPlugin ready for use.
    pub fn finish(mut self) -> errors::Result<WasmPlugin> {
        let mut import_object = wasmer::Imports::new();
        import_object.register_namespace("env", self.env);

        let instance = Instance::new(&mut self.store, &self.module, &import_object)?;

        for mut env in self.envs.into_iter() {
            env.update_exports(&instance.exports, &mut self.store);
        }

        Ok(WasmPlugin {
            inner: WasmPluginInner {
                instance,
                garbage: self.garbage,
            },
            store: self.store,
        })
    }
}

/// A marker trait for Fn types who's arguments and return type can be
/// serialized and are thus safe to import into a plugin;
pub trait ImportableFnWithContext<C, Arglist> {
    #[doc(hidden)]
    fn has_arg() -> bool;
    #[doc(hidden)]
    fn has_return() -> bool;
    #[doc(hidden)]
    fn call_with_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ptr: usize,
        len: usize,
        ctx: &C,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>>;
    #[doc(hidden)]
    fn call_without_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ctx: &C,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>>;
}

impl<C, Args, ReturnType, F> ImportableFnWithContext<C, Args> for F
where
    F: Fn(&C, Args) -> ReturnType,
    Args: Deserializable,
    ReturnType: Serializable,
{
    fn has_arg() -> bool {
        true
    }
    fn has_return() -> bool {
        std::mem::size_of::<ReturnType>() > 0
    }
    fn call_with_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ptr: usize,
        len: usize,
        ctx: &C,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        let message = message_buffer.read_message(ptr, len, store);
        let result = self(ctx, Args::deserialize(&message)?);
        if std::mem::size_of::<ReturnType>() > 0 {
            // No need to write anything for ZSTs
            let message = result.serialize()?;
            Ok(Some(message_buffer.write_message(&message, store)))
        } else {
            Ok(None)
        }
    }

    fn call_without_input(
        &self,
        _message_buffer: &mut MessageBuffer,
        _ctx: &C,
        _store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        unimplemented!("Requires argument")
    }
}

impl<C, ReturnType, F> ImportableFnWithContext<C, NoArgs> for F
where
    F: Fn(&C) -> ReturnType,
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
        _message_buffer: &mut MessageBuffer,
        _ptr: usize,
        _len: usize,
        _ctx: &C,
        _store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        unimplemented!("Must not supply argument")
    }

    fn call_without_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ctx: &C,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        let result = self(ctx);
        if std::mem::size_of::<ReturnType>() > 0 {
            // No need to write anything for ZSTs
            let message = result.serialize()?;
            Ok(Some(message_buffer.write_message(&message, store)))
        } else {
            Ok(None)
        }
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
    fn call_with_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ptr: usize,
        len: usize,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>>;
    #[doc(hidden)]
    fn call_without_input(
        &self,
        message_buffer: &mut MessageBuffer,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>>;
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
    fn call_with_input(
        &self,
        message_buffer: &mut MessageBuffer,
        ptr: usize,
        len: usize,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        let message = message_buffer.read_message(ptr, len, store);
        let result = self(Args::deserialize(&message)?);
        if std::mem::size_of::<ReturnType>() > 0 {
            let message = result.serialize()?;
            Ok(Some(message_buffer.write_message(&message, store)))
        } else {
            // No need to write anything for ZSTs
            Ok(None)
        }
    }

    fn call_without_input(
        &self,
        _message_buffer: &mut MessageBuffer,
        _store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
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
        _message_buffer: &mut MessageBuffer,
        _ptr: usize,
        _len: usize,
        _store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        unimplemented!("Must not supply argument")
    }

    fn call_without_input(
        &self,
        message_buffer: &mut MessageBuffer,
        store: &mut impl AsStoreMut,
    ) -> errors::Result<Option<FatPointer>> {
        let result = self();
        if std::mem::size_of::<ReturnType>() > 0 {
            // No need to write anything for ZSTs
            let message = result.serialize()?;
            Ok(Some(message_buffer.write_message(&message, store)))
        } else {
            Ok(None)
        }
    }
}

/// A loaded plugin
#[derive(Debug)]
pub struct WasmPlugin {
    store: Store,
    inner: WasmPluginInner,
}

#[derive(Debug)]
struct WasmPluginInner {
    instance: Instance,
    garbage: Arc<Mutex<Vec<FatPointer>>>,
}

enum OwnedOrRef<'a, T> {
    Owned(T),
    Ref(&'a T),
}

impl<T> AsRef<T> for OwnedOrRef<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            OwnedOrRef::Owned(v) => v,
            OwnedOrRef::Ref(r) => r,
        }
    }
}

#[doc(hidden)]
pub struct MessageBuffer<'a> {
    memory: &'a Memory,
    allocator: OwnedOrRef<'a, TypedFunction<u32, u32>>,
    garbage: Vec<FatPointer>,
}

impl<'a> MessageBuffer<'a> {
    fn write_message(&mut self, message: &[u8], store: &mut impl AsStoreMut) -> FatPointer {
        let len = message.len() as u32;

        let ptr = self.allocator.as_ref().call(store, len as u32).unwrap();

        unsafe {
            let mem = self.memory.view(store);
            let data = mem.data_unchecked_mut();
            data[ptr as usize..ptr as usize + len as usize].copy_from_slice(message);
        }

        let mut fat_ptr = FatPointer(0);
        fat_ptr.set_ptr(ptr);
        fat_ptr.set_len(len);
        self.garbage.push(FatPointer(fat_ptr.0));
        fat_ptr
    }

    fn read_message(&self, ptr: usize, len: usize, store: &impl AsStoreRef) -> Vec<u8> {
        let mut buff: Vec<u8> = vec![0; len];
        unsafe {
            let mem = self.memory.view(store);
            let data = mem.data_unchecked();
            buff.copy_from_slice(&data[ptr..ptr + len]);
        }
        buff
    }

    fn read_message_from_fat_pointer(&self, fat_ptr: u64, store: &impl AsStoreRef) -> Vec<u8> {
        unsafe {
            let mem = self.memory.view(store);
            let data = mem.data_unchecked();
            let fat_ptr = FatPointer(fat_ptr);
            let mut buff: Vec<u8> = vec![0; fat_ptr.len() as usize];
            buff.copy_from_slice(
                &data[fat_ptr.ptr() as usize..fat_ptr.ptr() as usize + fat_ptr.len() as usize],
            );
            buff
        }
    }
}

impl WasmPluginInner {
    fn message_buffer(&self, store: &impl AsStoreRef) -> errors::Result<MessageBuffer> {
        Ok(MessageBuffer {
            memory: self.instance.exports.get_memory("memory")?,
            allocator: OwnedOrRef::Owned(
                self.instance
                    .exports
                    .get::<Function>("allocate_message_buffer")?
                    .typed(store)?,
            ),
            garbage: vec![],
        })
    }

    fn call_function_raw(
        &self,
        fn_name: &str,
        input_buffer: Option<FatPointer>,
        mut store: impl AsStoreMut,
    ) -> errors::Result<Vec<u8>> {
        let f = self
            .instance
            .exports
            .get_function(&format!("wasm_plugin_exported__{}", fn_name))
            .unwrap_or_else(|_| panic!("Unable to find function {}", fn_name));

        let ptr = if let Some(fat_ptr) = input_buffer {
            f.typed::<(u32, u32), u64>(&store)?.call(
                &mut store,
                fat_ptr.ptr() as u32,
                fat_ptr.len() as u32,
            )?
        } else {
            f.typed::<(), u64>(&store)?.call(&mut store)?
        };
        let result = self
            .message_buffer(&store)?
            .read_message_from_fat_pointer(ptr, &store);

        let mut garbage: Vec<_> = self.garbage.lock().unwrap().drain(..).collect();

        if FatPointer(ptr).len() > 0 {
            garbage.push(FatPointer(ptr));
        }
        if !garbage.is_empty() {
            let f = self
                .instance
                .exports
                .get_function("free_message_buffer")
                .unwrap_or_else(|_| panic!("Unable to find function 'free_message_buffer'"))
                .typed::<(u32, u32), ()>(&store)?;
            for fat_ptr in garbage {
                f.call(&mut store, fat_ptr.ptr() as u32, fat_ptr.len() as u32)?
            }
        }

        Ok(result)
    }
}

impl WasmPlugin {
    /// Call a function exported by the plugin.
    ///
    /// Deserialization of the return value depends on the type being known
    /// at the call site.
    pub fn call_function<ReturnType>(&mut self, fn_name: &str) -> errors::Result<ReturnType>
    where
        ReturnType: Deserializable,
    {
        let buff = self
            .inner
            .call_function_raw(fn_name, None, &mut self.store)?;
        ReturnType::deserialize(&buff)
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
        let mut buffer = self.inner.message_buffer(&self.store)?;
        let ptr = buffer.write_message(&message, &mut self.store);

        let buff = self
            .inner
            .call_function_raw(fn_name, Some(ptr), &mut self.store)?;
        drop(buffer);
        ReturnType::deserialize(&buff)
    }
}

#[cfg(feature = "inject_getrandom")]
fn getrandom_shim(mut env: FunctionEnvMut<Env<()>>, ptr: u32, len: u32) {
    let (data, store) = env.data_and_store_mut();
    if let Some(memory) = data.memory_ref() {
        let view: MemoryView = memory.view(&store);
        let mut buff: Vec<u8> = vec![0; len as usize];
        getrandom::getrandom(&mut buff).unwrap();
        for (dst, src) in unsafe { view.data_unchecked_mut() }
            [ptr as usize..ptr as usize + len as usize]
            .iter_mut()
            .zip(buff)
        {
            *dst = src;
        }
    }
}
