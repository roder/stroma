//! Persistence module for Stroma's Reciprocal Persistence Network.
//!
//! This module implements the bot discovery registry that enables
//! bots to find each other for chunk distribution and recovery.

pub mod health;
pub mod registry;
pub mod write_blocking;

pub use health::{HealthStatus, ReplicationHealth};
pub use registry::{PersistenceRegistry, RegistryEntry, SizeBucket};
pub use write_blocking::{
    ChunkReplicationStatus, ReplicationHealth, WriteBlockingManager, WriteBlockingState,
};
