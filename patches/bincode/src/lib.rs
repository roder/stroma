//! Compatibility wrapper for unmaintained bincode crate
//!
//! This is a local patch that redirects to the maintained bincode2 fork.
//! See: RUSTSEC-2025-0141
//!
//! The bincode maintainers ceased development permanently. This wrapper
//! uses bincode2 as a drop-in replacement, which is actively maintained
//! by the Pravega team and maintains API compatibility.

// Re-export all public items from bincode2
pub use bincode2::{
    config,
    deserialize,
    deserialize_from,
    deserialize_from_custom,
    deserialize_in_place,
    serialize,
    serialize_into,
    serialized_size,
    Config,
    Error,
    ErrorKind,
    LengthOption,
    Result,
    BincodeRead,
    IoReader,
    SliceReader,
    DeserializerAcceptor,
    SerializerAcceptor,
};
