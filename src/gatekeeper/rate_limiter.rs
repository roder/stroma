//! Rate Limiting for Trust Actions (GAP-03)
//!
//! Implements progressive cooldown for trust operations to prevent abuse.
//! Per TODO.md line 922-924 and phase1-review-report.md line 234-237.
//!
//! ## Cooldown Tiers
//!
//! - 1st action: immediate (0 seconds)
//! - 2nd action: 1 minute (60 seconds)
//! - 3rd action: 5 minutes (300 seconds)
//! - 4th action: 1 hour (3600 seconds)
//! - 5th+ action: 24 hours (86400 seconds)
//!
//! ## Applies To
//!
//! - `/invite` - Invite new members
//! - `/vouch` - Vouch for members
//! - `/flag` - Flag members for violations
//! - `/propose` - Propose configuration changes
//!
//! ## Implementation Notes
//!
//! - Per-member tracking using `MemberHash`
//! - Action counts reset after cooldown period expires
//! - Thread-safe for concurrent access

use crate::freenet::contract::MemberHash;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Trust action types subject to rate limiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustAction {
    /// Invite a new member to the group
    Invite,
    /// Vouch for a member's admission
    Vouch,
    /// Flag a member for violation
    Flag,
    /// Propose a configuration change
    Propose,
}

impl TrustAction {
    /// Get the display name for this action (for error messages).
    pub fn name(&self) -> &'static str {
        match self {
            TrustAction::Invite => "invite",
            TrustAction::Vouch => "vouch",
            TrustAction::Flag => "flag",
            TrustAction::Propose => "propose",
        }
    }
}

/// Cooldown duration for each action tier.
///
/// - Tier 0 (1st action): 0 seconds (immediate)
/// - Tier 1 (2nd action): 60 seconds (1 minute)
/// - Tier 2 (3rd action): 300 seconds (5 minutes)
/// - Tier 3 (4th action): 3600 seconds (1 hour)
/// - Tier 4+ (5th+ action): 86400 seconds (24 hours)
const COOLDOWN_TIERS: &[u64] = &[
    0,     // 1st action: immediate
    60,    // 2nd action: 1 minute
    300,   // 3rd action: 5 minutes
    3600,  // 4th action: 1 hour
    86400, // 5th+ action: 24 hours
];

/// Rate limiter state for a single member and action type.
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Number of actions performed in current window
    action_count: u32,
    /// Timestamp of the last action
    last_action: SystemTime,
}

/// Rate limiter for trust actions.
///
/// Tracks action counts per member and enforces progressive cooldown.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// State per member per action type
    /// Key: (MemberHash, TrustAction) -> RateLimitState
    state: Arc<RwLock<HashMap<(MemberHash, TrustAction), RateLimitState>>>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a member can perform an action.
    ///
    /// Returns `Ok(())` if allowed, or `Err(duration)` with the remaining cooldown time.
    ///
    /// # Arguments
    ///
    /// * `member` - The member attempting the action
    /// * `action` - The action to perform
    ///
    /// # Returns
    ///
    /// - `Ok(())` if action is allowed
    /// - `Err(Duration)` with remaining cooldown time if blocked
    pub fn check_rate_limit(
        &self,
        member: &MemberHash,
        action: TrustAction,
    ) -> Result<(), Duration> {
        let now = SystemTime::now();
        let state = self.state.read().unwrap();

        if let Some(limit_state) = state.get(&(*member, action)) {
            let cooldown_secs = get_cooldown_duration(limit_state.action_count);
            let cooldown = Duration::from_secs(cooldown_secs);
            let elapsed = now
                .duration_since(limit_state.last_action)
                .unwrap_or(Duration::ZERO);

            if elapsed < cooldown {
                let remaining = cooldown - elapsed;
                return Err(remaining);
            }
        }

        Ok(())
    }

    /// Record that a member performed an action.
    ///
    /// This should be called after the action succeeds.
    /// Updates the action count and timestamp.
    ///
    /// # Arguments
    ///
    /// * `member` - The member who performed the action
    /// * `action` - The action that was performed
    pub fn record_action(&self, member: &MemberHash, action: TrustAction) {
        let now = SystemTime::now();
        let mut state = self.state.write().unwrap();

        let key = (*member, action);
        let limit_state = state.entry(key).or_insert(RateLimitState {
            action_count: 0,
            last_action: now,
        });

        let cooldown_secs = get_cooldown_duration(limit_state.action_count);
        let cooldown = Duration::from_secs(cooldown_secs);
        let elapsed = now
            .duration_since(limit_state.last_action)
            .unwrap_or(Duration::ZERO);

        // Reset count if cooldown has expired
        if elapsed >= cooldown {
            limit_state.action_count = 1;
        } else {
            limit_state.action_count += 1;
        }

        limit_state.last_action = now;
    }

    /// Get the remaining cooldown time for a member and action.
    ///
    /// Returns `None` if no cooldown is active (action is allowed).
    /// Returns `Some(Duration)` with the remaining time if blocked.
    ///
    /// # Arguments
    ///
    /// * `member` - The member to check
    /// * `action` - The action to check
    pub fn get_remaining_cooldown(
        &self,
        member: &MemberHash,
        action: TrustAction,
    ) -> Option<Duration> {
        self.check_rate_limit(member, action).err()
    }

    /// Reset the rate limit state for a member and action.
    ///
    /// This is useful for testing or administrative actions.
    ///
    /// # Arguments
    ///
    /// * `member` - The member to reset
    /// * `action` - The action to reset
    pub fn reset(&self, member: &MemberHash, action: TrustAction) {
        let mut state = self.state.write().unwrap();
        state.remove(&(*member, action));
    }

    /// Reset all rate limit state.
    ///
    /// This is useful for testing.
    pub fn reset_all(&self) {
        let mut state = self.state.write().unwrap();
        state.clear();
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the cooldown duration for a given action count.
///
/// Returns the cooldown in seconds based on the tier.
///
/// # Arguments
///
/// * `action_count` - The number of actions already performed
///
/// # Returns
///
/// Cooldown duration in seconds for the NEXT action
fn get_cooldown_duration(action_count: u32) -> u64 {
    let index = action_count as usize;
    if index >= COOLDOWN_TIERS.len() {
        // 5th+ action: use the maximum tier
        COOLDOWN_TIERS[COOLDOWN_TIERS.len() - 1]
    } else {
        COOLDOWN_TIERS[index]
    }
}

/// Format a duration as a human-readable string for error messages.
///
/// Examples:
/// - "1 second"
/// - "2 minutes"
/// - "1 hour"
/// - "1 day"
///
/// # Arguments
///
/// * `duration` - The duration to format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs >= 86400 {
        let days = secs / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    } else if secs >= 3600 {
        let hours = secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else if secs >= 60 {
        let minutes = secs / 60;
        format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
    } else {
        format!("{} second{}", secs, if secs == 1 { "" } else { "s" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread::sleep;

    fn test_member() -> MemberHash {
        MemberHash::from_bytes(&[1u8; 32])
    }

    fn test_member_2() -> MemberHash {
        MemberHash::from_bytes(&[2u8; 32])
    }

    #[test]
    fn test_first_action_immediate() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // First action should be immediate
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
    }

    #[test]
    fn test_second_action_blocked_1_minute() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // First action
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
        limiter.record_action(&member, TrustAction::Invite);

        // Second action should be blocked
        let result = limiter.check_rate_limit(&member, TrustAction::Invite);
        assert!(result.is_err());

        let remaining = result.unwrap_err();
        // Should be approximately 60 seconds
        assert!(remaining.as_secs() >= 59 && remaining.as_secs() <= 60);
    }

    #[test]
    fn test_third_action_blocked_5_minutes() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // First action
        limiter.record_action(&member, TrustAction::Invite);

        // Wait for cooldown to expire (simulated by resetting)
        sleep(Duration::from_millis(10));

        // Second action (manually advance state)
        {
            let mut state = limiter.state.write().unwrap();
            let key = (member, TrustAction::Invite);
            if let Some(s) = state.get_mut(&key) {
                s.action_count = 2; // Simulate second action completed
                s.last_action = SystemTime::now();
            }
        }

        // Third action should be blocked for 5 minutes
        let result = limiter.check_rate_limit(&member, TrustAction::Invite);
        assert!(result.is_err());

        let remaining = result.unwrap_err();
        // Should be approximately 300 seconds
        assert!(remaining.as_secs() >= 299 && remaining.as_secs() <= 300);
    }

    #[test]
    fn test_fourth_action_blocked_1_hour() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Manually set state to 3 actions
        {
            let mut state = limiter.state.write().unwrap();
            state.insert(
                (member, TrustAction::Invite),
                RateLimitState {
                    action_count: 3,
                    last_action: SystemTime::now(),
                },
            );
        }

        // Fourth action should be blocked for 1 hour
        let result = limiter.check_rate_limit(&member, TrustAction::Invite);
        assert!(result.is_err());

        let remaining = result.unwrap_err();
        // Should be approximately 3600 seconds
        assert!(remaining.as_secs() >= 3599 && remaining.as_secs() <= 3600);
    }

    #[test]
    fn test_fifth_action_blocked_24_hours() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Manually set state to 4 actions
        {
            let mut state = limiter.state.write().unwrap();
            state.insert(
                (member, TrustAction::Invite),
                RateLimitState {
                    action_count: 4,
                    last_action: SystemTime::now(),
                },
            );
        }

        // Fifth action should be blocked for 24 hours
        let result = limiter.check_rate_limit(&member, TrustAction::Invite);
        assert!(result.is_err());

        let remaining = result.unwrap_err();
        // Should be approximately 86400 seconds
        assert!(remaining.as_secs() >= 86399 && remaining.as_secs() <= 86400);
    }

    #[test]
    fn test_per_member_isolation() {
        let limiter = RateLimiter::new();
        let member1 = test_member();
        let member2 = test_member_2();

        // Member 1 performs action
        limiter.record_action(&member1, TrustAction::Invite);

        // Member 1 should be rate limited
        assert!(limiter
            .check_rate_limit(&member1, TrustAction::Invite)
            .is_err());

        // Member 2 should NOT be rate limited
        assert!(limiter
            .check_rate_limit(&member2, TrustAction::Invite)
            .is_ok());
    }

    #[test]
    fn test_per_action_isolation() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Member performs invite
        limiter.record_action(&member, TrustAction::Invite);

        // Invite should be rate limited
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_err());

        // Vouch should NOT be rate limited
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Vouch)
            .is_ok());
    }

    #[test]
    fn test_cooldown_expiry_resets_count() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Manually set state to expired cooldown
        {
            let mut state = limiter.state.write().unwrap();
            let old_time = SystemTime::now() - Duration::from_secs(61);
            state.insert(
                (member, TrustAction::Invite),
                RateLimitState {
                    action_count: 1,
                    last_action: old_time,
                },
            );
        }

        // Should be allowed (cooldown expired)
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());

        // Record action
        limiter.record_action(&member, TrustAction::Invite);

        // Verify count was reset (should be at count 1, not 2)
        {
            let state = limiter.state.read().unwrap();
            let key = (member, TrustAction::Invite);
            assert_eq!(state.get(&key).unwrap().action_count, 1);
        }
    }

    #[test]
    fn test_reset_clears_state() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Perform action and get rate limited
        limiter.record_action(&member, TrustAction::Invite);
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_err());

        // Reset
        limiter.reset(&member, TrustAction::Invite);

        // Should be allowed again
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
    }

    #[test]
    fn test_reset_all_clears_everything() {
        let limiter = RateLimiter::new();
        let member1 = test_member();
        let member2 = test_member_2();

        // Both members perform actions
        limiter.record_action(&member1, TrustAction::Invite);
        limiter.record_action(&member2, TrustAction::Vouch);

        // Both should be rate limited
        assert!(limiter
            .check_rate_limit(&member1, TrustAction::Invite)
            .is_err());
        assert!(limiter
            .check_rate_limit(&member2, TrustAction::Vouch)
            .is_err());

        // Reset all
        limiter.reset_all();

        // Both should be allowed again
        assert!(limiter
            .check_rate_limit(&member1, TrustAction::Invite)
            .is_ok());
        assert!(limiter
            .check_rate_limit(&member2, TrustAction::Vouch)
            .is_ok());
    }

    #[test]
    fn test_get_remaining_cooldown() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // No cooldown initially
        assert!(limiter
            .get_remaining_cooldown(&member, TrustAction::Invite)
            .is_none());

        // Record action
        limiter.record_action(&member, TrustAction::Invite);

        // Should have remaining cooldown
        let remaining = limiter.get_remaining_cooldown(&member, TrustAction::Invite);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() >= 59);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(1)), "1 second");
        assert_eq!(format_duration(Duration::from_secs(30)), "30 seconds");
        assert_eq!(format_duration(Duration::from_secs(60)), "1 minute");
        assert_eq!(format_duration(Duration::from_secs(120)), "2 minutes");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1 hour");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2 hours");
        assert_eq!(format_duration(Duration::from_secs(86400)), "1 day");
        assert_eq!(format_duration(Duration::from_secs(172800)), "2 days");
    }

    #[test]
    fn test_trust_action_names() {
        assert_eq!(TrustAction::Invite.name(), "invite");
        assert_eq!(TrustAction::Vouch.name(), "vouch");
        assert_eq!(TrustAction::Flag.name(), "flag");
        assert_eq!(TrustAction::Propose.name(), "propose");
    }

    #[test]
    fn test_cooldown_tier_calculation() {
        assert_eq!(get_cooldown_duration(0), 0); // Before 1st action (immediate)
        assert_eq!(get_cooldown_duration(1), 60); // Before 2nd action (1 min)
        assert_eq!(get_cooldown_duration(2), 300); // Before 3rd action (5 min)
        assert_eq!(get_cooldown_duration(3), 3600); // Before 4th action (1 hour)
        assert_eq!(get_cooldown_duration(4), 86400); // Before 5th action (24 hours)
        assert_eq!(get_cooldown_duration(5), 86400); // Before 6th+ action (capped)
        assert_eq!(get_cooldown_duration(100), 86400); // way beyond (capped)
    }

    #[test]
    fn test_rapid_fire_actions_blocked() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // First action succeeds
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
        limiter.record_action(&member, TrustAction::Invite);

        // Rapid-fire attempts should all be blocked
        for _ in 0..10 {
            assert!(limiter
                .check_rate_limit(&member, TrustAction::Invite)
                .is_err());
        }
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let limiter = Arc::new(RateLimiter::new());
        let member = test_member();

        // Record first action
        limiter.record_action(&member, TrustAction::Invite);

        // Spawn multiple threads trying to check rate limit
        let mut handles = vec![];
        for _ in 0..10 {
            let limiter_clone = Arc::clone(&limiter);
            let handle = thread::spawn(move || {
                let _ = limiter_clone.check_rate_limit(&member, TrustAction::Invite);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should still be rate limited after concurrent access
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_err());
    }

    #[test]
    fn test_default_rate_limiter() {
        let limiter1 = RateLimiter::new();
        let limiter2 = RateLimiter::default();

        let member = test_member();

        // Both should behave the same
        assert!(limiter1
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
        assert!(limiter2
            .check_rate_limit(&member, TrustAction::Invite)
            .is_ok());
    }

    #[test]
    fn test_get_remaining_cooldown_after_limit_expires() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Set state to expired cooldown
        {
            let mut state = limiter.state.write().unwrap();
            let old_time = SystemTime::now() - Duration::from_secs(61);
            state.insert(
                (member, TrustAction::Invite),
                RateLimitState {
                    action_count: 1,
                    last_action: old_time,
                },
            );
        }

        // Should have no remaining cooldown (expired)
        assert!(limiter
            .get_remaining_cooldown(&member, TrustAction::Invite)
            .is_none());
    }

    #[test]
    fn test_cooldown_tier_boundary_values() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Test exact boundary between tiers
        // After 1st action (count=1), should wait 60 seconds
        limiter.record_action(&member, TrustAction::Invite);
        let remaining = limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .unwrap_err();
        assert!(remaining.as_secs() >= 59 && remaining.as_secs() <= 60);
    }

    #[test]
    fn test_multiple_actions_different_types() {
        let limiter = RateLimiter::new();
        let member = test_member();

        // Record different action types
        limiter.record_action(&member, TrustAction::Invite);
        limiter.record_action(&member, TrustAction::Vouch);
        limiter.record_action(&member, TrustAction::Flag);
        limiter.record_action(&member, TrustAction::Propose);

        // All should be independently rate limited
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Invite)
            .is_err());
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Vouch)
            .is_err());
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Flag)
            .is_err());
        assert!(limiter
            .check_rate_limit(&member, TrustAction::Propose)
            .is_err());
    }

    #[test]
    fn test_format_duration_edge_cases() {
        // Test edge cases for duration formatting
        assert_eq!(format_duration(Duration::from_secs(0)), "0 seconds");
        assert_eq!(format_duration(Duration::from_secs(59)), "59 seconds");
        assert_eq!(format_duration(Duration::from_secs(61)), "1 minute");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59 minutes");
        assert_eq!(format_duration(Duration::from_secs(3601)), "1 hour");
        assert_eq!(format_duration(Duration::from_secs(86399)), "23 hours");
        assert_eq!(format_duration(Duration::from_secs(86401)), "1 day");
    }

    // Property tests for rate limiting
    proptest! {
        #[test]
        fn prop_cooldown_increases_monotonically(action_count in 0u32..10) {
            let cooldown1 = get_cooldown_duration(action_count);
            let cooldown2 = get_cooldown_duration(action_count + 1);

            // Cooldown should never decrease (monotonically increasing or stable)
            prop_assert!(cooldown2 >= cooldown1);
        }

        #[test]
        fn prop_rate_limiter_deterministic(
            byte1 in 0u8..255,
            byte2 in 0u8..255,
        ) {
            let limiter = RateLimiter::new();
            let member1 = MemberHash::from_bytes(&[byte1; 32]);
            let member2 = MemberHash::from_bytes(&[byte2; 32]);

            // Record action for member1
            limiter.record_action(&member1, TrustAction::Invite);

            // Check rate limit multiple times - must be deterministic
            let result1 = limiter.check_rate_limit(&member1, TrustAction::Invite);
            let result2 = limiter.check_rate_limit(&member1, TrustAction::Invite);
            let result3 = limiter.check_rate_limit(&member1, TrustAction::Invite);

            prop_assert_eq!(result1.is_ok(), result2.is_ok());
            prop_assert_eq!(result2.is_ok(), result3.is_ok());

            // member2 should not be affected (isolation) - unless member1 == member2
            if byte1 != byte2 {
                prop_assert!(limiter.check_rate_limit(&member2, TrustAction::Invite).is_ok());
            }
        }

        #[test]
        fn prop_progressive_cooldown_enforced(
            num_actions in 1u32..7,
        ) {
            let limiter = RateLimiter::new();
            let member = test_member();

            // Manually set action count
            {
                let mut state = limiter.state.write().unwrap();
                state.insert(
                    (member, TrustAction::Invite),
                    RateLimitState {
                        action_count: num_actions,
                        last_action: SystemTime::now(),
                    },
                );
            }

            // Check rate limit
            let result = limiter.check_rate_limit(&member, TrustAction::Invite);

            // Should be rate limited after any action
            if num_actions > 0 {
                prop_assert!(result.is_err());

                // Cooldown should match expected tier
                let expected_cooldown = get_cooldown_duration(num_actions);
                let actual_remaining = result.unwrap_err().as_secs();
                prop_assert!(actual_remaining >= expected_cooldown - 1); // Allow 1s tolerance
            }
        }

        #[test]
        fn prop_action_isolation(
            action1_count in 0u32..5,
            action2_count in 0u32..5,
        ) {
            let limiter = RateLimiter::new();
            let member = test_member();

            // Record different numbers of different actions
            for _ in 0..action1_count {
                limiter.record_action(&member, TrustAction::Invite);
                std::thread::sleep(std::time::Duration::from_millis(61));
            }

            for _ in 0..action2_count {
                limiter.record_action(&member, TrustAction::Vouch);
                std::thread::sleep(std::time::Duration::from_millis(61));
            }

            // State for each action should be independent
            let state = limiter.state.read().unwrap();

            if action1_count > 0 {
                let invite_state = state.get(&(member, TrustAction::Invite));
                prop_assert!(invite_state.is_some());
            }

            if action2_count > 0 {
                let vouch_state = state.get(&(member, TrustAction::Vouch));
                prop_assert!(vouch_state.is_some());
            }
        }
    }
}
