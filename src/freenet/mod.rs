//! Embedded Freenet kernel integration for Stroma.
//!
//! This module provides embedded Freenet node functionality with:
//! - In-process kernel (not external service)
//! - Real-time state stream monitoring (NOT polling)
//! - Two-layer architecture (trust state + persistence)
//! - Mock-friendly trait abstractions for testing

pub mod contract;
pub mod embedded_kernel;
pub mod state_stream;
pub mod traits;

pub use contract::TrustContract;
pub use embedded_kernel::EmbeddedKernel;
pub use state_stream::StateStream;
pub use traits::FreenetClient;
