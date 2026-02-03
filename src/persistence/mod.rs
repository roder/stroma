//! Persistence module for Stroma's Reciprocal Persistence Network.
//!
//! This module implements the bot discovery registry that enables
//! bots to find each other for chunk distribution and recovery.

pub mod registry;

pub use registry::{PersistenceRegistry, RegistryEntry, SizeBucket};
