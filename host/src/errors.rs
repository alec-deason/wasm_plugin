pub enum WasmPluginError {
    WasmerCompileError(wasmer::CompileError),
    WasmerInstantiationError(wasmer::InstantiationError),
    WasmerRuntimeError(wasmer::RuntimeError),
    IoError(std::io::Error),
    SerializationError,
    DeserializationError,
}

impl std::error::Error for WasmPluginError {}

impl core::fmt::Debug for WasmPluginError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl core::fmt::Display for WasmPluginError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WasmPluginError::WasmerCompileError(e) => e.fmt(f),
            WasmPluginError::WasmerInstantiationError(e) => e.fmt(f),
            WasmPluginError::WasmerRuntimeError(e) => e.fmt(f),
            WasmPluginError::IoError(e) => e.fmt(f),

            WasmPluginError::SerializationError => write!(f, "There was a problem serializing the argument to the function call"),
            WasmPluginError::DeserializationError=> write!(f, "There was a problem deserializing the value returned by the plugin function. This almost certainly means that the type at the call site does not match the type in the plugin's function signature."),
        }
    }
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

pub type Result<T> = std::result::Result<T, WasmPluginError>;
