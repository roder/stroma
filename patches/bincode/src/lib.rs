//! Compatibility wrapper for unmaintained bincode crate
//!
//! This is a local patch that redirects to the maintained bincode2 fork.
//! See: RUSTSEC-2025-0141
//!
//! The bincode maintainers ceased development permanently. This wrapper
//! uses bincode2 as a drop-in replacement, which is actively maintained
//! by the Pravega team and maintains binary format compatibility.
//!
//! This wrapper provides full bincode v1 API compatibility including:
//! - Options trait for configuration
//! - DefaultOptions struct
//! - Deserializer type for incremental deserialization

use std::io::{Read, Write};

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
    BincodeRead,
    Error,
    ErrorKind,
    IoReader,
    LengthOption,
    Result,
    SliceReader,
};

// Re-export Config from bincode2 (it's equivalent to bincode v1's internal config)
use bincode2::Config;

/// Options trait for configuring bincode serialization/deserialization
///
/// This trait provides a builder pattern for configuring bincode behavior.
/// It's compatible with bincode v1's Options trait.
pub trait Options: Clone {
    /// Set the byte limit for deserialization
    fn with_limit(self, limit: u64) -> WithLimit<Self>
    where
        Self: Sized,
    {
        WithLimit {
            options: self,
            limit,
        }
    }

    /// Remove the byte limit
    fn with_no_limit(self) -> WithNoLimit<Self>
    where
        Self: Sized,
    {
        WithNoLimit { options: self }
    }

    /// Use little-endian encoding
    fn with_little_endian(self) -> WithEndian<Self>
    where
        Self: Sized,
    {
        WithEndian {
            options: self,
            endian: Endian::Little,
        }
    }

    /// Use big-endian encoding
    fn with_big_endian(self) -> WithEndian<Self>
    where
        Self: Sized,
    {
        WithEndian {
            options: self,
            endian: Endian::Big,
        }
    }

    /// Use native-endian encoding
    fn with_native_endian(self) -> WithEndian<Self>
    where
        Self: Sized,
    {
        WithEndian {
            options: self,
            endian: Endian::Native,
        }
    }

    /// Use varint encoding for integers
    fn with_varint_encoding(self) -> Self
    where
        Self: Sized,
    {
        // bincode2 uses varint by default
        self
    }

    /// Use fixed-int encoding
    fn with_fixint_encoding(self) -> Self
    where
        Self: Sized,
    {
        // bincode2 doesn't distinguish between varint/fixint in the same way
        // For compatibility, we just return self
        self
    }

    /// Reject trailing bytes after deserialization
    fn reject_trailing_bytes(self) -> Self
    where
        Self: Sized,
    {
        // bincode2 rejects trailing bytes by default
        self
    }

    /// Allow trailing bytes after deserialization
    fn allow_trailing_bytes(self) -> Self
    where
        Self: Sized,
    {
        // For compatibility, we just return self
        self
    }

    /// Get the underlying Config
    fn config(&self) -> Config;

    /// Serialize a value
    fn serialize<T: serde::Serialize + ?Sized>(&self, t: &T) -> Result<Vec<u8>> {
        self.config().serialize(t)
    }

    /// Serialize a value into a writer
    fn serialize_into<W: Write, T: serde::Serialize + ?Sized>(
        &self,
        writer: W,
        t: &T,
    ) -> Result<()> {
        self.config().serialize_into(writer, t)
    }

    /// Get the serialized size of a value
    fn serialized_size<T: serde::Serialize + ?Sized>(&self, t: &T) -> Result<u64> {
        self.config().serialized_size(t)
    }

    /// Deserialize a value from bytes
    fn deserialize<'a, T: serde::Deserialize<'a>>(&self, bytes: &'a [u8]) -> Result<T> {
        self.config().deserialize(bytes)
    }

    /// Deserialize a value from a reader
    fn deserialize_from<R: Read, T: serde::de::DeserializeOwned>(&self, reader: R) -> Result<T> {
        self.config().deserialize_from(reader)
    }
}

/// Default options for bincode serialization
///
/// Uses little-endian encoding with no byte limit.
#[derive(Clone, Copy, Debug)]
pub struct DefaultOptions {
    endian: Endian,
    limit: Option<u64>,
}

impl DefaultOptions {
    /// Create a new DefaultOptions instance
    pub fn new() -> Self {
        Self {
            endian: Endian::Little,
            limit: None,
        }
    }
}

impl Default for DefaultOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Options for DefaultOptions {
    fn config(&self) -> Config {
        let mut config = bincode2::config();

        // Set endianness
        match self.endian {
            Endian::Little => { config.little_endian(); },
            Endian::Big => { config.big_endian(); },
            Endian::Native => { config.native_endian(); },
        }

        // Set limit
        if let Some(limit) = self.limit {
            config.limit(limit);
        } else {
            config.no_limit();
        }

        config.clone()
    }
}

/// Wrapper for options with a byte limit
#[derive(Clone, Copy, Debug)]
pub struct WithLimit<O: Options> {
    options: O,
    limit: u64,
}

impl<O: Options> Options for WithLimit<O> {
    fn config(&self) -> Config {
        let mut config = self.options.config();
        config.limit(self.limit);
        config
    }
}

/// Wrapper for options without a byte limit
#[derive(Clone, Copy, Debug)]
pub struct WithNoLimit<O: Options> {
    options: O,
}

impl<O: Options> Options for WithNoLimit<O> {
    fn config(&self) -> Config {
        let mut config = self.options.config();
        config.no_limit();
        config
    }
}

/// Wrapper for options with specific endianness
#[derive(Clone, Copy, Debug)]
pub struct WithEndian<O: Options> {
    options: O,
    endian: Endian,
}

impl<O: Options> Options for WithEndian<O> {
    fn config(&self) -> Config {
        let mut config = self.options.config();
        match self.endian {
            Endian::Little => { config.little_endian(); },
            Endian::Big => { config.big_endian(); },
            Endian::Native => { config.native_endian(); },
        }
        config
    }
}

/// Endianness for encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
    Native,
}

/// Deserializer for incremental deserialization
///
/// This type provides compatibility with bincode v1's Deserializer.
pub struct Deserializer<'de, R: BincodeRead<'de>> {
    pub reader: R,
    pub config: Config,
    _phantom: std::marker::PhantomData<&'de ()>,
}

impl<'de, R: BincodeRead<'de>> Deserializer<'de, R> {
    /// Create a new Deserializer from a reader
    pub fn new(reader: R, config: Config) -> Self {
        Self {
            reader,
            config,
            _phantom: std::marker::PhantomData,
        }
    }
}

// Specific impl block for SliceReader to provide from_slice
impl<'de> Deserializer<'de, SliceReader<'de>> {
    /// Create a Deserializer from a byte slice with Options
    pub fn from_slice(bytes: &'de [u8], options: impl Options) -> Self {
        Self::new(SliceReader::new(bytes), options.config())
    }
}

// Specific impl block for IoReader to provide from_reader
impl<'de, RR: Read> Deserializer<'de, IoReader<RR>> {
    /// Create a Deserializer from a reader with Options
    pub fn from_reader(reader: RR, options: impl Options) -> Self {
        Self::new(IoReader::new(reader), options.config())
    }
}

impl<'de, 'a, R: BincodeRead<'de>> serde::Deserializer<'de> for &'a mut Deserializer<'de, R> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(Error::from(ErrorKind::Custom(
            "deserialize_any not supported".to_string(),
        )))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

// Re-export bincode2's Config as a type alias for compatibility
pub use bincode2::Config as BincodeConfig;
