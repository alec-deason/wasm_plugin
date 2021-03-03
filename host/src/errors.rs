/// Error returned by WasmPlugin when loading plugins or calling functions.
pub enum WasmPluginError {
    /// A problem compiling the plugin's WASM source
    WasmerCompileError(wasmer::CompileError),
    /// A problem instantiating the Wasmer runtime
    WasmerInstantiationError(wasmer::InstantiationError),
    /// A problem interacting with the plugin
    WasmerRuntimeError(wasmer::RuntimeError),
    /// A problem loading the plugin's source from disk
    IoError(std::io::Error),
    /// A problems serializing an argument to send to one of the plugin's
    /// functions.
    SerializationError,
    /// A problem deserializing the return value of a call to one of the
    /// plugin's functions. This almost always represents a type mismatch
    /// between the callsite in the host and the function signature in the
    /// glugin.
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
