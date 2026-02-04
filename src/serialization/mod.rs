//! CBOR serialization for Freenet contract state
//!
//! Per `.beads/serialization-format.bead`:
//! - Freenet contract state: CBOR (compact, deterministic, cross-language)
//! - Persistence fragments: opaque bytes (already encrypted)
//! - Signal messages: Protobuf (Signal's native format)

use ciborium::{from_reader, into_writer};
use serde::{Deserialize, Serialize};
use std::io;

/// Serialization error type
#[derive(Debug)]
pub enum SerializationError {
    /// CBOR encoding/decoding error
    Cbor(ciborium::ser::Error<io::Error>),
    /// IO error
    Io(io::Error),
}

impl From<ciborium::ser::Error<io::Error>> for SerializationError {
    fn from(err: ciborium::ser::Error<io::Error>) -> Self {
        SerializationError::Cbor(err)
    }
}

impl From<ciborium::de::Error<io::Error>> for SerializationError {
    fn from(err: ciborium::de::Error<io::Error>) -> Self {
        // Convert deserialization error to serialization error
        SerializationError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CBOR deserialization error: {:?}", err),
        ))
    }
}

impl From<io::Error> for SerializationError {
    fn from(err: io::Error) -> Self {
        SerializationError::Io(err)
    }
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationError::Cbor(e) => write!(f, "CBOR error: {:?}", e),
            SerializationError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SerializationError {}

/// Trait for types that can be serialized to/from CBOR bytes
pub trait CborSerializable: Serialize + for<'de> Deserialize<'de> {
    /// Serialize to CBOR bytes
    fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let mut bytes = Vec::new();
        into_writer(self, &mut bytes)?;
        Ok(bytes)
    }

    /// Deserialize from CBOR bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError>
    where
        Self: Sized,
    {
        from_reader(bytes).map_err(Into::into)
    }

    /// Serialize to canonical CBOR bytes for deterministic hashing
    fn to_canonical_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        // For now, use standard serialization
        // TODO: Implement canonical mode via ciborium::value::CanonicalValue
        self.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        field1: String,
        field2: u64,
    }

    impl CborSerializable for TestStruct {}

    #[test]
    fn test_roundtrip_serialization() {
        let original = TestStruct {
            field1: "test".to_string(),
            field2: 42,
        };

        let bytes = original.to_bytes().unwrap();
        let recovered = TestStruct::from_bytes(&bytes).unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_deterministic_serialization() {
        let data = TestStruct {
            field1: "test".to_string(),
            field2: 42,
        };

        let bytes1 = data.to_canonical_bytes().unwrap();
        let bytes2 = data.to_canonical_bytes().unwrap();

        assert_eq!(bytes1, bytes2, "Canonical serialization must be deterministic");
    }
}
