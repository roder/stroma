//! Distinct Validator Ratio (DVR) calculation.
//!
//! Per mesh-health-metric.bead:
//! - DVR = Distinct_Validators / floor(N/4)
//! - Three-tier health: 🔴 <33%, 🟡 33-66%, 🟢 >66%
//! - Validators with non-overlapping voucher sets maximize network resilience

use crate::freenet::contract::MemberHash;
use crate::freenet::trust_contract::TrustNetworkState;
use std::collections::HashSet;

/// Health status based on DVR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Unhealthy: 0-33% DVR (🔴 Red)
    Unhealthy,
    /// Developing: 33-66% DVR (🟡 Yellow)
    Developing,
    /// Healthy: 66-100% DVR (🟢 Green)
    Healthy,
}

impl HealthStatus {
    /// Get emoji representation.
    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Unhealthy => "🔴",
            HealthStatus::Developing => "🟡",
            HealthStatus::Healthy => "🟢",
        }
    }

    /// Get status name.
    pub fn name(&self) -> &'static str {
        match self {
            HealthStatus::Unhealthy => "Unhealthy",
            HealthStatus::Developing => "Developing",
            HealthStatus::Healthy => "Healthy",
        }
    }

    /// Get bot behavior description.
    pub fn behavior(&self) -> &'static str {
        match self {
            HealthStatus::Unhealthy => "Actively suggest introductions",
            HealthStatus::Developing => "Opportunistic suggestions",
            HealthStatus::Healthy => "Maintenance mode",
        }
    }
}

/// DVR calculation result.
#[derive(Debug, Clone, PartialEq)]
pub struct DvrResult {
    /// DVR ratio (0.0 to 1.0).
    pub ratio: f32,
    /// Number of distinct validators found.
    pub distinct_validators: usize,
    /// Maximum possible distinct validators (N/4).
    pub max_possible: usize,
    /// Network size (total members).
    pub network_size: usize,
    /// Health status.
    pub health: HealthStatus,
}

impl DvrResult {
    /// Get percentage (0-100).
    pub fn percentage(&self) -> f32 {
        self.ratio * 100.0
    }
}

/// Calculate Distinct Validator Ratio.
///
/// Per mesh-health-metric.bead:
/// - DVR = Distinct_Validators / floor(N/4)
/// - Bootstrap exception: networks with < 4 members return 100%
pub fn calculate_dvr(state: &TrustNetworkState) -> DvrResult {
    let network_size = state.members.len();

    // Bootstrap exception: too small to measure meaningfully
    if network_size < 4 {
        return DvrResult {
            ratio: 1.0,
            distinct_validators: 0,
            max_possible: 0,
            network_size,
            health: HealthStatus::Healthy,
        };
    }

    let max_possible = network_size / 4;
    if max_possible == 0 {
        return DvrResult {
            ratio: 1.0,
            distinct_validators: 0,
            max_possible: 0,
            network_size,
            health: HealthStatus::Healthy,
        };
    }

    let distinct_count = count_distinct_validators(state);
    let ratio = (distinct_count as f32 / max_possible as f32).min(1.0);
    let health = health_status(ratio);

    DvrResult {
        ratio,
        distinct_validators: distinct_count,
        max_possible,
        network_size,
        health,
    }
}

/// Count Validators with non-overlapping voucher sets.
///
/// Per mesh-health-metric.bead:
/// - Greedy algorithm: select Validators whose voucher sets don't overlap
/// - Sort by vouch count descending (prefer more connected first)
/// - A Validator needs >= 3 vouches (per trust model)
pub fn count_distinct_validators(state: &TrustNetworkState) -> usize {
    // Collect Validators (members with >= 3 vouches)
    let validators: Vec<MemberHash> = state
        .members
        .iter()
        .filter(|m| {
            let vouch_count = state.vouches.get(m).map(|v| v.len()).unwrap_or(0);
            vouch_count >= 3
        })
        .copied()
        .collect();

    if validators.is_empty() {
        return 0;
    }

    // Sort by vouch count descending (prefer more connected first)
    let mut sorted_validators = validators.clone();
    sorted_validators.sort_by_key(|v| {
        let vouch_count = state
            .vouches
            .get(v)
            .map(|vouchers| vouchers.len())
            .unwrap_or(0);
        std::cmp::Reverse(vouch_count)
    });

    // Greedy selection: pick Validators with non-overlapping voucher sets
    let mut distinct = Vec::new();
    let mut used_vouchers = HashSet::new();

    for validator in sorted_validators {
        let vouchers: HashSet<MemberHash> = state
            .vouches
            .get(&validator)
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default();

        // Check if any voucher already used
        if vouchers.is_disjoint(&used_vouchers) {
            distinct.push(validator);
            used_vouchers.extend(vouchers);
        }
    }

    distinct.len()
}

/// Get health status from DVR ratio.
///
/// Per mesh-health-metric.bead:
/// - Unhealthy: 0-33% (🔴 Red)
/// - Developing: 33-66% (🟡 Yellow)
/// - Healthy: 66-100% (🟢 Green)
pub fn health_status(dvr: f32) -> HealthStatus {
    match dvr {
        d if d < 0.33 => HealthStatus::Unhealthy,
        d if d < 0.66 => HealthStatus::Developing,
        _ => HealthStatus::Healthy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::contract::MemberHash;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    #[test]
    fn test_bootstrap_network() {
        // Network with < 4 members should return 100% DVR
        let mut state = TrustNetworkState::new();
        state.members.insert(test_member(1));
        state.members.insert(test_member(2));
        state.members.insert(test_member(3));

        let result = calculate_dvr(&state);
        assert_eq!(result.ratio, 1.0);
        assert_eq!(result.health, HealthStatus::Healthy);
    }

    #[test]
    fn test_small_network_healthy() {
        // 12 members, max possible = 3, actual = 2 distinct
        // DVR = 2/3 = 66.7% (Healthy)
        let mut state = TrustNetworkState::new();

        // Add 12 members
        for i in 1..=12 {
            state.members.insert(test_member(i));
        }

        // Create 2 distinct Validators
        // V1 vouched by {1, 2, 3}
        let v1 = test_member(10);
        let mut v1_vouchers = HashSet::new();
        v1_vouchers.insert(test_member(1));
        v1_vouchers.insert(test_member(2));
        v1_vouchers.insert(test_member(3));
        state.vouches.insert(v1, v1_vouchers);

        // V2 vouched by {4, 5, 6} (no overlap with V1)
        let v2 = test_member(11);
        let mut v2_vouchers = HashSet::new();
        v2_vouchers.insert(test_member(4));
        v2_vouchers.insert(test_member(5));
        v2_vouchers.insert(test_member(6));
        state.vouches.insert(v2, v2_vouchers);

        let result = calculate_dvr(&state);
        assert_eq!(result.distinct_validators, 2);
        assert_eq!(result.max_possible, 3);
        assert!((result.ratio - 0.667).abs() < 0.01);
        assert_eq!(result.health, HealthStatus::Healthy);
    }

    #[test]
    fn test_medium_network_developing() {
        // 20 members, max possible = 5, actual = 2 distinct
        // DVR = 2/5 = 40% (Developing)
        let mut state = TrustNetworkState::new();

        // Add 20 members
        for i in 1..=20 {
            state.members.insert(test_member(i));
        }

        // Create 2 distinct Validators
        let v1 = test_member(18);
        let mut v1_vouchers = HashSet::new();
        v1_vouchers.insert(test_member(1));
        v1_vouchers.insert(test_member(2));
        v1_vouchers.insert(test_member(3));
        state.vouches.insert(v1, v1_vouchers);

        let v2 = test_member(19);
        let mut v2_vouchers = HashSet::new();
        v2_vouchers.insert(test_member(4));
        v2_vouchers.insert(test_member(5));
        v2_vouchers.insert(test_member(6));
        state.vouches.insert(v2, v2_vouchers);

        let result = calculate_dvr(&state);
        assert_eq!(result.distinct_validators, 2);
        assert_eq!(result.max_possible, 5);
        assert_eq!(result.ratio, 0.4);
        assert_eq!(result.health, HealthStatus::Developing);
    }

    #[test]
    fn test_unhealthy_network() {
        // 20 members, max possible = 5, actual = 1 distinct
        // DVR = 1/5 = 20% (Unhealthy)
        let mut state = TrustNetworkState::new();

        // Add 20 members
        for i in 1..=20 {
            state.members.insert(test_member(i));
        }

        // Create 1 distinct Validator
        let v1 = test_member(18);
        let mut v1_vouchers = HashSet::new();
        v1_vouchers.insert(test_member(1));
        v1_vouchers.insert(test_member(2));
        v1_vouchers.insert(test_member(3));
        state.vouches.insert(v1, v1_vouchers);

        let result = calculate_dvr(&state);
        assert_eq!(result.distinct_validators, 1);
        assert_eq!(result.max_possible, 5);
        assert_eq!(result.ratio, 0.2);
        assert_eq!(result.health, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_overlapping_vouchers_not_distinct() {
        // Validators with overlapping voucher sets should not be counted as distinct
        let mut state = TrustNetworkState::new();

        // Add 12 members
        for i in 1..=12 {
            state.members.insert(test_member(i));
        }

        // V1 vouched by {1, 2, 3}
        let v1 = test_member(10);
        let mut v1_vouchers = HashSet::new();
        v1_vouchers.insert(test_member(1));
        v1_vouchers.insert(test_member(2));
        v1_vouchers.insert(test_member(3));
        state.vouches.insert(v1, v1_vouchers);

        // V2 vouched by {1, 4, 5} (shares voucher 1 with V1)
        let v2 = test_member(11);
        let mut v2_vouchers = HashSet::new();
        v2_vouchers.insert(test_member(1)); // Overlap!
        v2_vouchers.insert(test_member(4));
        v2_vouchers.insert(test_member(5));
        state.vouches.insert(v2, v2_vouchers);

        let result = count_distinct_validators(&state);
        // Only V1 should be counted (greedy selects highest vouch count first)
        assert_eq!(result, 1);
    }

    #[test]
    fn test_health_status_boundaries() {
        assert_eq!(health_status(0.0), HealthStatus::Unhealthy);
        assert_eq!(health_status(0.32), HealthStatus::Unhealthy);
        assert_eq!(health_status(0.33), HealthStatus::Developing);
        assert_eq!(health_status(0.50), HealthStatus::Developing);
        assert_eq!(health_status(0.65), HealthStatus::Developing);
        assert_eq!(health_status(0.66), HealthStatus::Healthy);
        assert_eq!(health_status(1.0), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status_emoji() {
        assert_eq!(HealthStatus::Unhealthy.emoji(), "🔴");
        assert_eq!(HealthStatus::Developing.emoji(), "🟡");
        assert_eq!(HealthStatus::Healthy.emoji(), "🟢");
    }

    #[test]
    fn test_dvr_percentage() {
        let result = DvrResult {
            ratio: 0.75,
            distinct_validators: 3,
            max_possible: 4,
            network_size: 16,
            health: HealthStatus::Healthy,
        };
        assert_eq!(result.percentage(), 75.0);
    }

    #[test]
    fn test_no_validators() {
        // Network with no members with >= 3 vouches
        let mut state = TrustNetworkState::new();

        // Add 12 members
        for i in 1..=12 {
            state.members.insert(test_member(i));
        }

        // Add some members with < 3 vouches
        let m1 = test_member(10);
        let mut m1_vouchers = HashSet::new();
        m1_vouchers.insert(test_member(1));
        m1_vouchers.insert(test_member(2));
        state.vouches.insert(m1, m1_vouchers);

        let result = calculate_dvr(&state);
        assert_eq!(result.distinct_validators, 0);
        assert_eq!(result.ratio, 0.0);
        assert_eq!(result.health, HealthStatus::Unhealthy);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    proptest! {
        /// Property test: DVR ≤ 1.0 for all graphs
        /// For any valid network configuration, DVR ratio must be in [0.0, 1.0]
        #[test]
        fn prop_dvr_bounded(
            network_size in 0usize..100,
            num_validators in 0usize..30,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members to network
            for i in 0..network_size as u8 {
                state.members.insert(test_member(i));
            }

            // Add validators with non-overlapping voucher sets
            let mut voucher_pool = 100u8;
            for i in 0..num_validators.min(network_size) {
                let validator = test_member(i as u8);
                let mut vouchers = HashSet::new();

                // Give each validator 3 unique vouchers
                for _ in 0..3 {
                    vouchers.insert(test_member(voucher_pool));
                    voucher_pool = voucher_pool.saturating_add(1);
                }

                state.vouches.insert(validator, vouchers);
            }

            let result = calculate_dvr(&state);

            // DVR must be in [0.0, 1.0]
            prop_assert!(result.ratio >= 0.0, "DVR ratio {} is negative", result.ratio);
            prop_assert!(result.ratio <= 1.0, "DVR ratio {} exceeds 1.0", result.ratio);

            // Distinct validators cannot exceed actual validator count
            prop_assert!(
                result.distinct_validators <= num_validators.min(network_size),
                "Distinct validators {} exceeds actual validators {}",
                result.distinct_validators,
                num_validators.min(network_size)
            );
        }

        /// Property test: Distinct validators have disjoint voucher sets
        /// When count_distinct_validators returns N, those N validators must have
        /// completely non-overlapping voucher sets
        #[test]
        fn prop_distinct_validators_disjoint_vouchers(
            num_validators in 3usize..20,
            vouchers_per_validator in 3usize..8,
        ) {
            let mut state = TrustNetworkState::new();

            // Add enough members for the network
            let network_size = num_validators + (num_validators * vouchers_per_validator);
            for i in 0..network_size as u8 {
                state.members.insert(test_member(i));
            }

            // Create validators with non-overlapping voucher sets
            let mut voucher_pool = 100u8;
            for i in 0..num_validators {
                let validator = test_member(i as u8);
                let mut vouchers = HashSet::new();

                for _ in 0..vouchers_per_validator {
                    vouchers.insert(test_member(voucher_pool));
                    voucher_pool = voucher_pool.saturating_add(1);
                }

                state.vouches.insert(validator, vouchers);
            }

            let distinct_count = count_distinct_validators(&state);

            // Verify that the selected distinct validators have disjoint voucher sets
            // by re-implementing the greedy algorithm and checking
            let mut validators: Vec<MemberHash> = state
                .members
                .iter()
                .filter(|m| {
                    let vouch_count = state.vouches.get(m).map(|v| v.len()).unwrap_or(0);
                    vouch_count >= 3
                })
                .copied()
                .collect();

            validators.sort_by_key(|v| {
                let vouch_count = state.vouches.get(v).map(|v| v.len()).unwrap_or(0);
                std::cmp::Reverse(vouch_count)
            });

            let mut used_vouchers = HashSet::new();
            let mut distinct_validators = Vec::new();

            for validator in validators {
                let vouchers: HashSet<MemberHash> = state
                    .vouches
                    .get(&validator)
                    .map(|v| v.iter().copied().collect())
                    .unwrap_or_default();

                if vouchers.is_disjoint(&used_vouchers) {
                    distinct_validators.push(validator);
                    used_vouchers.extend(vouchers);
                }
            }

            // All distinct validators must have disjoint voucher sets
            for i in 0..distinct_validators.len() {
                for j in (i + 1)..distinct_validators.len() {
                    let vouchers_i: HashSet<MemberHash> = state
                        .vouches
                        .get(&distinct_validators[i])
                        .map(|v| v.iter().copied().collect())
                        .unwrap_or_default();
                    let vouchers_j: HashSet<MemberHash> = state
                        .vouches
                        .get(&distinct_validators[j])
                        .map(|v| v.iter().copied().collect())
                        .unwrap_or_default();

                    prop_assert!(
                        vouchers_i.is_disjoint(&vouchers_j),
                        "Distinct validators {:?} and {:?} have overlapping voucher sets",
                        distinct_validators[i],
                        distinct_validators[j]
                    );
                }
            }

            prop_assert_eq!(distinct_count, distinct_validators.len());
        }

        /// Property test: DVR calculation consistency
        /// DVR = distinct_validators / max_possible
        /// where max_possible = floor(N/4) for N >= 4
        #[test]
        fn prop_dvr_calculation_consistency(
            network_size in 4usize..50,
            num_validators in 0usize..20,
        ) {
            let mut state = TrustNetworkState::new();

            // Add members
            for i in 0..network_size as u8 {
                state.members.insert(test_member(i));
            }

            // Add validators with unique voucher sets
            let mut voucher_pool = 100u8;
            for i in 0..num_validators.min(network_size) {
                let validator = test_member(i as u8);
                let mut vouchers = HashSet::new();

                for _ in 0..3 {
                    vouchers.insert(test_member(voucher_pool));
                    voucher_pool = voucher_pool.saturating_add(1);
                }

                state.vouches.insert(validator, vouchers);
            }

            let result = calculate_dvr(&state);
            let expected_max = network_size / 4;

            prop_assert_eq!(
                result.max_possible,
                expected_max,
                "max_possible should be floor(N/4) = floor({}/4) = {}",
                network_size,
                expected_max
            );

            // If we have distinct validators, verify the ratio calculation
            if result.max_possible > 0 {
                let expected_ratio = (result.distinct_validators as f32 / result.max_possible as f32).min(1.0);
                let ratio_diff = (result.ratio - expected_ratio).abs();
                prop_assert!(
                    ratio_diff < 0.001,
                    "DVR ratio {} does not match expected {} (diff: {})",
                    result.ratio,
                    expected_ratio,
                    ratio_diff
                );
            }
        }
    }
}
