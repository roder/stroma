//! Blind Matchmaker: Strategic Introduction Recommendations
//!
//! Implements DVR-optimized algorithm for suggesting introductions that strengthen
//! the trust network while maintaining privacy.
//!
//! Per blind-matchmaker-dvr.bead:
//! - Phase 0: DVR optimization (prioritize distinct Validators)
//! - Phase 1: MST fallback (connectivity optimization)
//! - Phase 2: Cluster bridging (connect islands)
//!
//! Uses Signal display names in user-facing messages (transient mapping).

pub mod display;
pub mod graph_analysis;
pub mod strategic_intro;

pub use display::{IntroductionMessage, resolve_display_name};
pub use graph_analysis::{ClusterId, TrustGraph, detect_clusters};
pub use strategic_intro::{Introduction, suggest_introductions};
