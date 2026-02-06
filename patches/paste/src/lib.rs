//! Compatibility wrapper for unmaintained paste crate
//!
//! This is a local patch that redirects to the maintained pastey fork.
//! See: RUSTSEC-2024-0436

// Re-export the paste macro from pastey
pub use pastey::paste;
