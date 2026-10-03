//! One JSON helper for every snapshot type.

use serde::{Serialize, de::DeserializeOwned};

/// Every snapshot serializes and deserializes the same way, so the conversion
/// lives in one place.
pub trait JsonSnapshot: Serialize + DeserializeOwned + Sized {
    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
