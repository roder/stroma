//! Gatekeeper module: Admission & Ejection Protocol
//!
//! Per architecture-decisions.bead:
//! - Admission: Vetting & admission logic
//! - Ejection: Immediate ejection (two triggers)
//! - Health Monitor: Continuous standing checks
//! - Audit Trail: Operator action logging (GAP-01)
//! - Rate Limiter: Progressive cooldown for trust actions (GAP-03)

pub mod audit_trail;
pub mod ejection;
pub mod health_monitor;
pub mod rate_limiter;

pub use audit_trail::{format_audit_log, query_audit_log, ActionType, AuditEntry, AuditQuery};
pub use ejection::{eject_member, should_eject, EjectionError, EjectionResult};
pub use health_monitor::HealthMonitor;
pub use rate_limiter::{format_duration, RateLimiter, TrustAction};
