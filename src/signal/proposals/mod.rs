// ! Proposal system for group governance.
//!
//! Per .beads/proposal-system.bead:
//! - Mandatory timeouts (min 1h, max 168h)
//! - Poll termination via PollTerminate
//! - Quorum + threshold checks
//! - GAP-02: NO individual votes persisted (only aggregates)
//! - Use Freenet state stream (NOT polling)

pub mod command;
pub mod duration_parse;
pub mod executor;
pub mod lifecycle;

pub use command::{parse_propose_args, ProposalSubcommand, ProposeArgs};
pub use duration_parse::parse_duration_to_secs;
pub use executor::execute_proposal;
pub use lifecycle::{create_proposal, monitor_proposals};
