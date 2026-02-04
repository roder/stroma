//! Gatekeeper module: Admission & Ejection Protocol
//!
//! Per architecture-decisions.bead:
//! - Admission: Vetting & admission logic
//! - Ejection: Immediate ejection (two triggers)
//! - Health Monitor: Continuous standing checks

pub mod ejection;
pub mod health_monitor;

pub use ejection::{eject_member, should_eject, EjectionError, EjectionResult};
pub use health_monitor::HealthMonitor;
