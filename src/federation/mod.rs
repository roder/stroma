//! Federation discovery for cross-group trust networks.
//!
//! Per FEDERATION.md: Phase 3 computes social anchors locally (no broadcast).
//! Phase 4+ will enable Shadow Beacon broadcast and PSI-CA handshakes.
//!
//! ## Social Anchor Discovery
//!
//! Groups discover each other via "social frequency" (overlapping validators):
//! - Hash top-N validators using Fibonacci buckets
//! - Groups with shared top-N → matching social anchors
//! - No admin coordination needed (emergent discovery)
//!
//! ## Privacy-Preserving Overlap
//!
//! PSI-CA (Private Set Intersection Cardinality) reveals ONLY overlap count,
//! not which specific members overlap (Phase 4+).
//!
//! ## Phase 3 Usage (Local Computation Only)
//!
//! ```rust
//! use stroma::federation::{calculate_social_anchors, generate_discovery_uris};
//! use stroma::freenet::contract::TrustContract;
//!
//! // Given a trust contract with members and vouches
//! let contract = TrustContract::new();
//! // ... add members and vouches via contract.apply_delta() ...
//!
//! // Compute social anchors at all Fibonacci buckets
//! let anchors = calculate_social_anchors(&contract);
//! for (bucket_size, anchor) in &anchors {
//!     println!("Bucket {}: {}", bucket_size, anchor);
//! }
//!
//! // Generate discovery URIs (Phase 4+ will publish these to Freenet)
//! let uris = generate_discovery_uris(&contract);
//! for uri in &uris {
//!     println!("URI (bucket {}): {}", uri.bucket_size, uri.uri);
//! }
//! ```
//!
//! ## Fibonacci Buckets
//!
//! Groups publish social anchors at multiple bucket sizes (3, 5, 8, 13, 21, ...):
//! - Small groups: fewer buckets (e.g., 20 members → buckets 3, 5, 8, 13)
//! - Large groups: more buckets (e.g., 200 members → all buckets up to 144)
//! - **Discovery property**: Groups with shared top-N validators produce
//!   MATCHING social anchors at bucket-N, enabling mutual discovery.

pub mod social_anchor;

pub use social_anchor::{
    calculate_social_anchors, generate_discovery_uris, DiscoveryUri, SocialAnchor,
    FIBONACCI_BUCKETS,
};
