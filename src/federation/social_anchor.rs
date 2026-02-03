//! Social Anchor computation for federation discovery.
//!
//! Per FEDERATION.md Phase 3: Compute locally, don't broadcast.
//! - Hash format: H(group_members_sorted)
//! - Fibonacci buckets (not percentiles) for fixed-count matching
//! - Groups with shared top-N validators produce MATCHING social anchors
//!
//! This enables emergent federation discovery without admin coordination.

use crate::freenet::contract::{MemberHash, TrustContract};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Social anchor hash for federation discovery.
///
/// Per FEDERATION.md: Groups with shared top-N validators hash to the same
/// social anchor, enabling emergent discovery without pre-coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SocialAnchor([u8; 32]);

impl SocialAnchor {
    /// Create from hash bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[..32]);
        Self(hash)
    }

    /// Get bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Convert to hex string for display/logging.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl std::fmt::Display for SocialAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Fibonacci buckets for social anchor computation.
///
/// Per FEDERATION.md: Fixed counts ensure groups of different sizes produce
/// MATCHING hashes at the same bucket if they share top-N validators.
///
/// Fibonacci sequence up to Signal's 1000-member limit.
pub const FIBONACCI_BUCKETS: &[usize] = &[3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987];

/// Discovery URI for a social anchor at a specific bucket size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryUri {
    /// Fibonacci bucket size (number of validators in anchor).
    pub bucket_size: usize,
    /// Social anchor hash.
    pub anchor: SocialAnchor,
    /// Freenet URI (freenet://stroma/discovery/{anchor}).
    pub uri: String,
}

impl DiscoveryUri {
    /// Create discovery URI for an anchor and bucket size.
    pub fn new(bucket_size: usize, anchor: SocialAnchor) -> Self {
        let uri = format!("freenet://stroma/discovery/{}", anchor.to_hex());
        Self {
            bucket_size,
            anchor,
            uri,
        }
    }
}

/// Member with effective vouch count for ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedMember {
    hash: MemberHash,
    vouch_count: usize,
}

/// Calculate effective vouch count for each member.
///
/// Per FEDERATION.md: Sort members by effective vouch count to identify
/// top validators for social anchor computation.
fn calculate_vouch_counts(contract: &TrustContract) -> HashMap<MemberHash, usize> {
    let mut counts = HashMap::new();

    // Count vouches received by each member
    for (voucher, vouchees) in &contract.vouches {
        // Only count vouches from active members
        if !contract.members().contains(voucher) {
            continue;
        }

        for vouchee in vouchees {
            if contract.members().contains(vouchee) {
                *counts.entry(*vouchee).or_insert(0) += 1;
            }
        }
    }

    // Ensure all members have an entry (even if 0 vouches)
    for member in contract.members() {
        counts.entry(*member).or_insert(0);
    }

    counts
}

/// Compute social anchor for a set of top validators.
///
/// Per FEDERATION.md: Hash sorted validators to create social anchor.
/// Deterministic ordering ensures groups with same top-N produce same hash.
fn compute_social_anchor(validators: &[MemberHash]) -> SocialAnchor {
    let mut hasher = Sha256::new();

    // Sort validators for deterministic ordering
    let mut sorted = validators.to_vec();
    sorted.sort();

    // Hash each validator
    for validator in sorted {
        hasher.update(validator.as_bytes());
    }

    SocialAnchor::from_bytes(&hasher.finalize())
}

/// Calculate all social anchors for a trust contract.
///
/// Per FEDERATION.md Phase 3: Compute locally, don't broadcast.
///
/// Returns social anchors at all Fibonacci bucket sizes the group can fill.
/// Larger groups publish at more buckets → more discovery chances.
///
/// # Arguments
/// * `contract` - Trust contract with members and vouches
///
/// # Returns
/// Vector of (bucket_size, social_anchor) tuples, sorted by bucket size.
pub fn calculate_social_anchors(contract: &TrustContract) -> Vec<(usize, SocialAnchor)> {
    // Calculate vouch counts for all members
    let vouch_counts = calculate_vouch_counts(contract);

    // Rank members by vouch count (descending)
    let mut ranked: Vec<RankedMember> = contract
        .members()
        .iter()
        .map(|&hash| RankedMember {
            hash,
            vouch_count: *vouch_counts.get(&hash).unwrap_or(&0),
        })
        .collect();

    // Sort by vouch count descending, then by hash for deterministic ordering
    ranked.sort_by(|a, b| {
        b.vouch_count
            .cmp(&a.vouch_count)
            .then_with(|| a.hash.cmp(&b.hash))
    });

    // Extract sorted member hashes (highest vouch count first)
    let sorted_members: Vec<MemberHash> = ranked.iter().map(|r| r.hash).collect();

    // Compute social anchors at all Fibonacci buckets we can fill
    FIBONACCI_BUCKETS
        .iter()
        .filter(|&&bucket| sorted_members.len() >= bucket)
        .map(|&bucket| {
            let top_validators = &sorted_members[..bucket];
            let anchor = compute_social_anchor(top_validators);
            (bucket, anchor)
        })
        .collect()
}

/// Generate discovery URIs for a trust contract.
///
/// Per FEDERATION.md: Generate URIs at all Fibonacci buckets for discovery.
/// Phase 3: Generate but don't publish. Phase 4+: Publish to Freenet.
///
/// # Arguments
/// * `contract` - Trust contract with members and vouches
///
/// # Returns
/// Vector of discovery URIs, sorted by bucket size.
pub fn generate_discovery_uris(contract: &TrustContract) -> Vec<DiscoveryUri> {
    calculate_social_anchors(contract)
        .into_iter()
        .map(|(bucket_size, anchor)| DiscoveryUri::new(bucket_size, anchor))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::contract::TrustDelta;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    fn create_test_contract_with_vouches() -> TrustContract {
        let mut contract = TrustContract::new();

        // Add 10 members
        for i in 1..=10 {
            contract.apply_delta(&TrustDelta::AddMember {
                member: test_member(i),
            });
        }

        // Member 1: 5 vouches (top validator)
        for i in 2..=6 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(1),
            });
        }

        // Member 2: 4 vouches
        for i in 3..=6 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(2),
            });
        }

        // Member 3: 3 vouches
        for i in 4..=6 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(3),
            });
        }

        // Member 4: 2 vouches
        for i in 5..=6 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(4),
            });
        }

        // Member 5: 1 vouch
        contract.apply_delta(&TrustDelta::AddVouch {
            voucher: test_member(6),
            vouchee: test_member(5),
        });

        // Members 6-10: 0 vouches

        contract
    }

    #[test]
    fn test_social_anchor_creation() {
        let bytes = [42u8; 32];
        let anchor = SocialAnchor::from_bytes(&bytes);
        assert_eq!(anchor.as_bytes(), &bytes);
    }

    #[test]
    fn test_social_anchor_hex_display() {
        let bytes = [0x12, 0x34, 0x56, 0x78];
        let mut full_bytes = [0u8; 32];
        full_bytes[..4].copy_from_slice(&bytes);
        let anchor = SocialAnchor::from_bytes(&full_bytes);
        let hex = anchor.to_hex();
        assert!(hex.starts_with("12345678"));
    }

    #[test]
    fn test_calculate_vouch_counts() {
        let contract = create_test_contract_with_vouches();
        let counts = calculate_vouch_counts(&contract);

        assert_eq!(counts.get(&test_member(1)), Some(&5)); // Top validator
        assert_eq!(counts.get(&test_member(2)), Some(&4));
        assert_eq!(counts.get(&test_member(3)), Some(&3));
        assert_eq!(counts.get(&test_member(4)), Some(&2));
        assert_eq!(counts.get(&test_member(5)), Some(&1));
        assert_eq!(counts.get(&test_member(6)), Some(&0)); // No vouches
    }

    #[test]
    fn test_compute_social_anchor_deterministic() {
        let validators = vec![test_member(1), test_member(2), test_member(3)];

        let anchor1 = compute_social_anchor(&validators);
        let anchor2 = compute_social_anchor(&validators);

        // Same validators → same anchor
        assert_eq!(anchor1, anchor2);
    }

    #[test]
    fn test_compute_social_anchor_order_independent() {
        let validators1 = vec![test_member(1), test_member(2), test_member(3)];
        let validators2 = vec![test_member(3), test_member(1), test_member(2)];

        let anchor1 = compute_social_anchor(&validators1);
        let anchor2 = compute_social_anchor(&validators2);

        // Order shouldn't matter (internally sorted)
        assert_eq!(anchor1, anchor2);
    }

    #[test]
    fn test_compute_social_anchor_different_validators() {
        let validators1 = vec![test_member(1), test_member(2), test_member(3)];
        let validators2 = vec![test_member(1), test_member(2), test_member(4)];

        let anchor1 = compute_social_anchor(&validators1);
        let anchor2 = compute_social_anchor(&validators2);

        // Different validators → different anchors
        assert_ne!(anchor1, anchor2);
    }

    #[test]
    fn test_calculate_social_anchors_fibonacci_buckets() {
        let contract = create_test_contract_with_vouches();
        let anchors = calculate_social_anchors(&contract);

        // 10 members → should fill buckets [3, 5, 8]
        assert_eq!(anchors.len(), 3);
        assert_eq!(anchors[0].0, 3); // Bucket size 3
        assert_eq!(anchors[1].0, 5); // Bucket size 5
        assert_eq!(anchors[2].0, 8); // Bucket size 8
    }

    #[test]
    fn test_calculate_social_anchors_uses_top_validators() {
        let contract = create_test_contract_with_vouches();
        let anchors = calculate_social_anchors(&contract);

        // Get the bucket-3 anchor (top 3 validators)
        let bucket3_anchor = anchors.iter().find(|(size, _)| *size == 3).unwrap().1;

        // Manually compute expected anchor from top 3 validators
        // Top 3: member 1 (5 vouches), member 2 (4 vouches), member 3 (3 vouches)
        let expected = compute_social_anchor(&[test_member(1), test_member(2), test_member(3)]);

        assert_eq!(bucket3_anchor, expected);
    }

    #[test]
    fn test_calculate_social_anchors_empty_contract() {
        let contract = TrustContract::new();
        let anchors = calculate_social_anchors(&contract);

        // No members → no anchors
        assert!(anchors.is_empty());
    }

    #[test]
    fn test_calculate_social_anchors_small_group() {
        let mut contract = TrustContract::new();

        // Add only 4 members
        for i in 1..=4 {
            contract.apply_delta(&TrustDelta::AddMember {
                member: test_member(i),
            });
        }

        let anchors = calculate_social_anchors(&contract);

        // 4 members → should only fill bucket [3]
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].0, 3);
    }

    #[test]
    fn test_generate_discovery_uris() {
        let contract = create_test_contract_with_vouches();
        let uris = generate_discovery_uris(&contract);

        // Should have 3 URIs (buckets 3, 5, 8)
        assert_eq!(uris.len(), 3);

        // Check URI format
        for uri in &uris {
            assert!(uri.uri.starts_with("freenet://stroma/discovery/"));
            assert_eq!(uri.uri.len(), "freenet://stroma/discovery/".len() + 64);
            // 64 hex chars
        }

        // Check bucket sizes
        assert_eq!(uris[0].bucket_size, 3);
        assert_eq!(uris[1].bucket_size, 5);
        assert_eq!(uris[2].bucket_size, 8);
    }

    #[test]
    fn test_discovery_uri_creation() {
        let anchor = compute_social_anchor(&[test_member(1)]);
        let uri = DiscoveryUri::new(3, anchor);

        assert_eq!(uri.bucket_size, 3);
        assert_eq!(uri.anchor, anchor);
        assert!(uri.uri.starts_with("freenet://stroma/discovery/"));
    }

    #[test]
    fn test_matching_social_anchors_different_groups() {
        // Simulate two groups with shared top validators

        // Group A: 10 members
        let mut contract_a = TrustContract::new();
        for i in 1..=10 {
            contract_a.apply_delta(&TrustDelta::AddMember {
                member: test_member(i),
            });
        }

        // Group B: 8 members (first 8 overlap with Group A)
        let mut contract_b = TrustContract::new();
        for i in 1..=8 {
            contract_b.apply_delta(&TrustDelta::AddMember {
                member: test_member(i),
            });
        }

        // Both groups have same vouches for top validators
        for i in 1..=5 {
            for j in 6..=8 {
                contract_a.apply_delta(&TrustDelta::AddVouch {
                    voucher: test_member(j),
                    vouchee: test_member(i),
                });
                contract_b.apply_delta(&TrustDelta::AddVouch {
                    voucher: test_member(j),
                    vouchee: test_member(i),
                });
            }
        }

        let anchors_a = calculate_social_anchors(&contract_a);
        let anchors_b = calculate_social_anchors(&contract_b);

        // Both should have bucket-3 and bucket-5
        let bucket3_a = anchors_a.iter().find(|(size, _)| *size == 3).unwrap().1;
        let bucket3_b = anchors_b.iter().find(|(size, _)| *size == 3).unwrap().1;

        // Shared top-3 validators → matching anchors
        assert_eq!(bucket3_a, bucket3_b);
    }

    #[test]
    fn test_social_anchor_ordering_by_vouch_count() {
        let mut contract = TrustContract::new();

        // Add 5 members
        for i in 1..=5 {
            contract.apply_delta(&TrustDelta::AddMember {
                member: test_member(i),
            });
        }

        // Member 5 has most vouches (should be top validator)
        for i in 1..=4 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(5),
            });
        }

        // Member 4 has second most
        for i in 1..=3 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(4),
            });
        }

        // Member 3 has third most
        for i in 1..=2 {
            contract.apply_delta(&TrustDelta::AddVouch {
                voucher: test_member(i),
                vouchee: test_member(3),
            });
        }

        let anchors = calculate_social_anchors(&contract);
        let bucket3_anchor = anchors.iter().find(|(size, _)| *size == 3).unwrap().1;

        // Top 3 should be members 5, 4, 3
        let expected = compute_social_anchor(&[test_member(5), test_member(4), test_member(3)]);

        assert_eq!(bucket3_anchor, expected);
    }
}
