pub trait Serializable {
    fn serialize(&self) -> Vec<u8>;
}
#[cfg(feature = "serialize_bincode")]
impl<T: serde::Serialize> Serializable for T {
    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
#[cfg(feature = "serialize_json")]
impl<T: serde::Serialize> Serializable for T {
    fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}
#[cfg(feature = "serialize_nanoserde_json")]
impl<T: nanoserde::SerJson> Serializable for T {
    fn serialize(&self) -> Vec<u8> {
        nanoserde::SerJson::serialize_json(self).as_bytes().to_vec()
    }
}

pub trait Deserializable {
    fn deserialize(data: &[u8]) -> Self;
}
#[cfg(feature = "serialize_bincode")]
impl<T: serde::de::DeserializeOwned + Clone> Deserializable for T {
    fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}
#[cfg(feature = "serialize_json")]
impl<T: serde::de::DeserializeOwned + Clone> Deserializable for T {
    fn deserialize(data: &[u8]) -> Self {
        serde_json::from_slice(data).unwrap()
    }
}
#[cfg(feature = "serialize_nanoserde_json")]
impl<T: nanoserde::DeJson> Deserializable for T {
    fn deserialize(data: &[u8]) -> Self {
        nanoserde::DeJson::deserialize_json(std::str::from_utf8(data).unwrap()).unwrap()
    }
}
