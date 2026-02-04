//! Matchmaker: Trust Network Health and Strategic Introductions
//!
//! Implements both DVR-based health metrics and strategic introduction recommendations.
//!
//! Per blind-matchmaker-dvr.bead and mesh-health-metric.bead:
//! - DVR (Distinct Validator Ratio) calculation and health status
//! - Cluster detection using Bridge Removal algorithm
//! - GAP-11: Cluster formation announcements
//! - Strategic introductions that strengthen the trust network
//! - Signal display name resolution for user-facing messages

// Health metrics and cluster analysis
pub mod cluster_detection;
pub mod dvr;

// Strategic introductions
pub mod display;
pub mod graph_analysis;
pub mod strategic_intro;

// Re-exports for health/cluster features
pub use cluster_detection::{detect_clusters, ClusterId, ClusterResult};
pub use dvr::{calculate_dvr, count_distinct_validators, health_status, DvrResult, HealthStatus};

// Re-exports for strategic introduction features
pub use display::{resolve_display_name, IntroductionMessage};
pub use graph_analysis::TrustGraph;
pub use strategic_intro::{suggest_introductions, Introduction};
