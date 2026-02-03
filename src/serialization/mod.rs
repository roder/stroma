//! CBOR serialization for Freenet contract state.
//!
//! Per serialization-format.bead:
//! - Use CBOR via `ciborium` (NOT JSON or bincode)
//! - Deterministic serialization for hashing
//! - Cross-language compatibility
//! - Efficient schema evolution with #[serde(default)]

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Serialization errors.
#[derive(Debug, Error)]
pub enum SerializationError {
    /// CBOR encoding failed.
    #[error("CBOR encoding failed: {0}")]
    Encode(String),

    /// CBOR decoding failed.
    #[error("CBOR decoding failed: {0}")]
    Decode(String),
}

/// Serialize to CBOR bytes.
///
/// Per serialization-format.bead: "CBOR for Freenet contract state"
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes)
        .map_err(|e| SerializationError::Encode(format!("{:?}", e)))?;
    Ok(bytes)
}

/// Deserialize from CBOR bytes.
pub fn from_cbor<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SerializationError> {
    ciborium::from_reader(bytes).map_err(|e| SerializationError::Decode(format!("{:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        value: u64,
        name: String,
    }

    #[test]
    fn test_cbor_roundtrip() {
        let original = TestStruct {
            value: 42,
            name: "test".to_string(),
        };
        let bytes = to_cbor(&original).unwrap();
        let recovered: TestStruct = from_cbor(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_cbor_deterministic() {
        let value = TestStruct {
            value: 123,
            name: "hello".to_string(),
        };
        let bytes1 = to_cbor(&value).unwrap();
        let bytes2 = to_cbor(&value).unwrap();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_cbor_backward_compatibility() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct V1 {
            field1: u32,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct V2 {
            field1: u32,
            #[serde(default)]
            field2: Option<String>,
        }

        let v1 = V1 { field1: 42 };
        let bytes = to_cbor(&v1).unwrap();

        // V2 can deserialize V1 data with default for new field
        let v2: V2 = from_cbor(&bytes).unwrap();
        assert_eq!(v2.field1, 42);
        assert_eq!(v2.field2, None);
    }
}
