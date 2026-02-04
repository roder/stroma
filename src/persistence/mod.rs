//! Persistence module for Stroma's Reciprocal Persistence Network.
//!
//! This module implements the bot discovery registry that enables
//! bots to find each other for chunk distribution and recovery.

pub mod chunks;
pub mod health;
pub mod recovery;
pub mod registry;
pub mod rendezvous;
pub mod write_blocking;

pub use chunks::{decrypt_and_reassemble, encrypt_and_chunk, Chunk, ChunkError, CHUNK_SIZE};
pub use health::{HealthStatus, ReplicationHealth};
pub use recovery::{
    recover_state, ChunkFetcher, RecoveredState, RecoveryConfig, RecoveryError, RecoveryStats,
    RegistryFetcher,
};
pub use registry::{PersistenceRegistry, RegistryEntry, SizeBucket};
pub use rendezvous::{compute_all_chunk_holders, compute_chunk_holders};
pub use write_blocking::{ChunkReplicationStatus, WriteBlockingManager, WriteBlockingState};
