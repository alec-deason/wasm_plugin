#[derive(Debug)]
pub enum WasmPluginError {
    WasmerCompileError(wasmer::CompileError),
    WasmerInstantiationError(wasmer::InstantiationError),
    WasmerRuntimeError(wasmer::RuntimeError),
    BincodeError(bincode::Error),
    IoError(std::io::Error),
}

impl From<std::io::Error> for WasmPluginError {
    fn from(e: std::io::Error) -> WasmPluginError {
        WasmPluginError::IoError(e)
    }
}

impl From<wasmer::CompileError> for WasmPluginError {
    fn from(e: wasmer::CompileError) -> WasmPluginError {
        WasmPluginError::WasmerCompileError(e)
    }
}

impl From<wasmer::InstantiationError> for WasmPluginError {
    fn from(e: wasmer::InstantiationError) -> WasmPluginError {
        WasmPluginError::WasmerInstantiationError(e)
    }
}

impl From<wasmer::RuntimeError> for WasmPluginError {
    fn from(e: wasmer::RuntimeError) -> WasmPluginError {
        WasmPluginError::WasmerRuntimeError(e)
    }
}

impl From<bincode::Error> for WasmPluginError {
    fn from(e: bincode::Error) -> WasmPluginError {
        WasmPluginError::BincodeError(e)
    }
}

pub type Result<T> = std::result::Result<T, WasmPluginError>;
