use std::path::Path;

use wasmer::{
    imports, Function, Global, Instance, LazyInit, Memory, MemoryView, Module, Store, Value,
    WasmerEnv,
};

mod errors;

#[derive(Clone)]
pub struct WasmPlugin {
    pub instance: Instance,
}

#[derive(WasmerEnv, Clone, Default, Debug)]
pub struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

impl WasmPlugin {
    pub fn load(path: impl AsRef<Path>) -> errors::Result<Self> {
        let wasm_src = std::fs::read(path)?;
        let store = Store::default();
        let import_object = imports! {
            "env" => { "__getrandom" => Function::new_native_with_env(&store, Env::default(), getrandom_shim), },
        };
        let module = Module::new(&store, wasm_src)?;

        let instance = Instance::new(&module, &import_object)?;
        Ok(Self { instance })
    }

    pub fn call_function_with_message<ReturnType, Args>(
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
        let message = bincode::serialize(args).map_err(|_| errors::WasmPluginError::SerializationError)?;
        let len = message.len() as i32;

        unsafe {
            let data = memory.data_unchecked_mut();
            data[memory_idx as usize..memory_idx as usize + len as usize].copy_from_slice(&message);
        }

        self.call_function(fn_name)
    }

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
        Ok(bincode::deserialize(&buff).map_err(|_| errors::WasmPluginError::DeserializationError)?)
    }
}

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
