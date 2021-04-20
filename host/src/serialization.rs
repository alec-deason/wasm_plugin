use crate::errors;

pub trait Serializable {
    fn serialize(&self) -> errors::Result<Vec<u8>>;
}
#[cfg(feature = "serialize_bincode")]
impl<T: serde::Serialize> Serializable for T {
    fn serialize(&self) -> errors::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|_| errors::WasmPluginError::SerializationError)
    }
}
#[cfg(feature = "serialize_json")]
impl<T: serde::Serialize> Serializable for T {
    fn serialize(&self) -> errors::Result<Vec<u8>> {
        serde_json::to_string(self).map_err(|_| errors::WasmPluginError::SerializationError)
    }
}
#[cfg(feature = "serialize_nanoserde_json")]
impl<T: nanoserde::SerJson> Serializable for T {
    fn serialize(&self) -> errors::Result<Vec<u8>> {
        Ok(nanoserde::SerJson::serialize_json(self).as_bytes().to_vec())
    }
}

pub trait Deserializable {
    fn deserialize(data: &[u8]) -> errors::Result<Self>
    where
        Self: Sized;
}
#[cfg(feature = "serialize_bincode")]
impl<T: serde::de::DeserializeOwned + Clone> Deserializable for T {
    fn deserialize(data: &[u8]) -> errors::Result<Self> {
        bincode::deserialize(data).map_err(|_| errors::WasmPluginError::DeserializationError)
    }
}
#[cfg(feature = "serialize_json")]
impl<T: serde::de::DeserializeOwned + Clone> Deserializable for T {
    fn deserialize(data: &[u8]) -> errors::Result<Self> {
        serde_json::from_str(self).map_err(|_| errors::WasmPluginError::DeserializationError)
    }
}
#[cfg(feature = "serialize_nanoserde_json")]
impl<T: nanoserde::DeJson> Deserializable for T {
    fn deserialize(data: &[u8]) -> errors::Result<Self> {
        nanoserde::DeJson::deserialize_json(
            std::str::from_utf8(data).map_err(|_| errors::WasmPluginError::DeserializationError)?,
        )
        .map_err(|_| errors::WasmPluginError::DeserializationError)
    }
}
