//! Stroma - Decentralized Trust Network Bot
//!
//! A Signal bot that enforces distributed vetting via Freenet contracts.
//!
//! Key principles:
//! - NO message history (ephemeral only)
//! - Protocol state only (~100KB)
//! - Freenet as source of truth
//! - Zero-knowledge proof verification
//!
//! See: .beads/signal-integration.bead, .beads/security-constraints.bead

pub mod signal;
