//! Signal Protocol Integration Module
//!
//! Implements Signal messaging bot with custom protocol store that:
//! - Stores ONLY protocol state (~100KB)
//! - NO message history
//! - NO contact database
//! - Ephemeral vetting conversations
//!
//! See: .beads/signal-integration.bead, .beads/security-constraints.bead § 10

pub mod traits;
pub mod store;
pub mod mock;
pub mod linking;
pub mod group;
pub mod pm;
pub mod polls;
pub mod bot;
pub mod bootstrap;

pub use traits::{SignalClient, SignalError, SignalResult};
pub use store::StromaProtocolStore;
pub use mock::MockSignalClient;
pub use bot::{StromaBot, BotConfig};
pub use bootstrap::{BootstrapManager, BootstrapState};
