//! Signal Protocol Integration Module
//!
//! Implements Signal messaging bot with custom protocol store that:
//! - Stores ONLY protocol state (~100KB)
//! - NO message history
//! - NO contact database
//! - Ephemeral vetting conversations
//!
//! See: .beads/signal-integration.bead, .beads/security-constraints.bead § 10

pub mod bootstrap;
pub mod bot;
pub mod group;
pub mod linking;
pub mod matchmaker;
pub mod mock;
pub mod pm;
pub mod polls;
pub mod proposals;
pub mod retry;
pub mod store;
pub mod traits;
pub mod vetting;

pub use bootstrap::{BootstrapManager, BootstrapState};
pub use bot::{BotConfig, StromaBot};
pub use mock::MockSignalClient;
pub use store::StromaProtocolStore;
pub use traits::{SignalClient, SignalError, SignalResult};
