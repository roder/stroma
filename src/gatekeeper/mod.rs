//! Gatekeeper module: Admission & Ejection Protocol
//!
//! Per architecture-decisions.bead:
//! - Admission: Vetting & admission logic
//! - Ejection: Immediate ejection (two triggers)
//! - Health Monitor: Continuous standing checks

pub mod health_monitor;

pub use health_monitor::HealthMonitor;
