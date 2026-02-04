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
            let vouch_count = state
                .vouches
                .get(m)
                .map(|v| v.len())
                .unwrap_or(0);
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
