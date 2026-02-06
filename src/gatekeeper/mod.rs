//! Gatekeeper module: Admission & Ejection Protocol
//!
//! Per architecture-decisions.bead:
//! - Admission: Vetting & admission logic
//! - Ejection: Immediate ejection (two triggers)
//! - Health Monitor: Continuous standing checks
//! - Audit Trail: Operator action logging (GAP-01)

pub mod audit_trail;
pub mod ejection;
pub mod health_monitor;

pub use audit_trail::{ActionType, AuditEntry, AuditQuery, format_audit_log, query_audit_log};
pub use ejection::{eject_member, should_eject, EjectionError, EjectionResult};
pub use health_monitor::HealthMonitor;
