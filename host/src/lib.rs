use std::path::Path;

use wasmer::{imports, MemoryView, Instance, Value, WasmerEnv, Memory, LazyInit, Store, Module, Function};
use anyhow::Result;


pub struct WASMPlugin {
    instance: Instance
}

#[derive(WasmerEnv, Clone, Default)]
pub struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

fn getrandom_shim(env: &Env, ptr: i32, len: i32) {
     if let Some(memory) = env.memory_ref() {
         let view: MemoryView<u8> = memory.view();
         let mut buff: Vec<u8> = vec![0; len as usize];
         getrandom::getrandom(&mut buff).unwrap();
         for (dst, src) in view[ptr as usize..ptr as usize + len as usize].iter().
zip(buff) {
             dst.set(src);
         }
     }
 }

impl WASMPlugin {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let wasm_src = std::fs::read(path)?;
		let store = Store::default();
         let import_object = imports! {
             "env" => { "__getrandom" => Function::new_native_with_env(&store, Env::default(), getrandom_shim), },
         };
         let module = Module::new(&store, wasm_src)?;

        let instance = Instance::new(&module, &import_object)?;
        Ok(Self { instance })
    }

    pub fn call_function_with_message<T, M>(&mut self, fn_name: &str, argument: &M) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Clone,
        M: serde::Serialize
    {
        let f = self.instance
            .exports
            .get_function(fn_name)
            .unwrap();

        let memory = self.instance.exports.get_memory("memory").unwrap();
        let view: MemoryView<u8> = memory.view();

        let message = bincode::serialize(argument)?;
        let len = message.len() as i32;
        for (src, dst) in message.iter().zip(&view[..len as usize]) {
            dst.set(*src);
        }
        let result_len = f.native::<(i32, i32, i32), i32>()?
        .call(0, len, 2048)?;

        let mut buff: Vec<u8> = Vec::with_capacity(result_len as usize);
        for c in &view[0..result_len as usize] {
            buff.push(c.get());
        }
        Ok(bincode::deserialize(&buff)?)
    }

    pub fn call_function<T>(&mut self, fn_name: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        let f = self.instance
            .exports
            .get_function(fn_name)
            .unwrap();

        let memory = self.instance.exports.get_memory("memory").unwrap();
        let view: MemoryView<u8> = memory.view();

        let result_len = f.native::<(i32, i32), i32>()?
        .call(0, 2048)?;

        let mut buff: Vec<u8> = Vec::with_capacity(result_len as usize);
        for c in &view[0..result_len as usize] {
            buff.push(c.get());
        }
        Ok(bincode::deserialize(&buff)?)
    }
}
