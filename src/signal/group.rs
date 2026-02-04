//! Group Management
//!
//! Bot manages Signal group membership based on Freenet trust state.
//! Core invariant: Every member must maintain ≥2 vouches from different clusters.
//!
//! See: .beads/signal-integration.bead § Bot Behavior Principles

use super::traits::*;

/// Group management operations
pub struct GroupManager<C: SignalClient> {
    client: C,
    group_id: GroupId,
}

impl<C: SignalClient> GroupManager<C> {
    pub fn new(client: C, group_id: GroupId) -> Self {
        Self { client, group_id }
    }

    /// Add member to Signal group
    ///
    /// MUST verify with Freenet first (never make Signal source of truth).
    pub async fn add_member(&self, member: &ServiceId) -> SignalResult<()> {
        // TODO: Verify with Freenet that member is vetted
        // See: .beads/security-constraints.bead § 2 (Freenet as Source of Truth)

        self.client.add_group_member(&self.group_id, member).await
    }

    /// Remove member from Signal group
    ///
    /// Called immediately when ejection triggers met (no delay).
    pub async fn remove_member(&self, member: &ServiceId) -> SignalResult<()> {
        self.client
            .remove_group_member(&self.group_id, member)
            .await
    }

    /// Announce member admission (hashes only, no names)
    pub async fn announce_admission(&self, member_hash: &str) -> SignalResult<()> {
        let message = format!("New member admitted: {}", member_hash);
        self.client
            .send_group_message(&self.group_id, &message)
            .await
    }

    /// Announce member ejection (hashes only, no reasons)
    pub async fn announce_ejection(&self, member_hash: &str) -> SignalResult<()> {
        let message = format!("Member ejected: {}", member_hash);
        self.client
            .send_group_message(&self.group_id, &message)
            .await
    }
}

/// Ejection triggers (both independent)
pub enum EjectionTrigger {
    /// Standing < 0 (effective_vouches - regular_flags)
    NegativeStanding {
        effective_vouches: u32,
        regular_flags: u32,
    },

    /// Effective vouches below minimum threshold
    BelowThreshold {
        effective_vouches: u32,
        min_threshold: u32,
    },
}

impl EjectionTrigger {
    /// Check if member should be ejected
    pub fn should_eject(
        all_vouchers: u32,
        all_flaggers: u32,
        voucher_flaggers: u32,
        min_threshold: u32,
    ) -> Option<Self> {
        let effective_vouches = all_vouchers - voucher_flaggers;
        let regular_flags = all_flaggers - voucher_flaggers;
        let standing = effective_vouches as i32 - regular_flags as i32;

        // Trigger 1: Negative standing
        if standing < 0 {
            return Some(EjectionTrigger::NegativeStanding {
                effective_vouches,
                regular_flags,
            });
        }

        // Trigger 2: Below threshold
        if effective_vouches < min_threshold {
            return Some(EjectionTrigger::BelowThreshold {
                effective_vouches,
                min_threshold,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::mock::MockSignalClient;

    #[tokio::test]
    async fn test_add_member() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        let manager = GroupManager::new(client.clone(), group.clone());
        manager.add_member(&member).await.unwrap();

        assert!(client.is_member(&group, &member));
    }

    #[tokio::test]
    async fn test_remove_member() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        client.add_group_member(&group, &member).await.unwrap();
        assert!(client.is_member(&group, &member));

        let manager = GroupManager::new(client.clone(), group.clone());
        manager.remove_member(&member).await.unwrap();

        assert!(!client.is_member(&group, &member));
    }

    #[tokio::test]
    async fn test_announce_admission() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        client.add_group_member(&group, &member).await.unwrap();

        let manager = GroupManager::new(client.clone(), group);
        manager.announce_admission("hash123").await.unwrap();

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
    }

    #[test]
    fn test_ejection_trigger_negative_standing() {
        // Alice vouched and flagged Bob
        // all_vouchers: 2 (Alice, Carol)
        // all_flaggers: 1 (Alice)
        // voucher_flaggers: 1 (Alice)
        // effective_vouches: 2 - 1 = 1
        // regular_flags: 1 - 1 = 0
        // standing: 1 - 0 = 1 (NOT negative)

        let trigger = EjectionTrigger::should_eject(2, 1, 1, 2);

        // Should eject due to below threshold, not negative standing
        assert!(matches!(
            trigger,
            Some(EjectionTrigger::BelowThreshold { .. })
        ));
    }

    #[test]
    fn test_ejection_trigger_below_threshold() {
        // Bob has only 1 vouch (below min_threshold of 2)
        let trigger = EjectionTrigger::should_eject(1, 0, 0, 2);

        assert!(matches!(
            trigger,
            Some(EjectionTrigger::BelowThreshold {
                effective_vouches: 1,
                min_threshold: 2
            })
        ));
    }

    #[test]
    fn test_no_ejection_trigger() {
        // Bob has 3 vouches, 1 flag, no contradictions
        let trigger = EjectionTrigger::should_eject(3, 1, 0, 2);
        assert!(trigger.is_none());
    }

    #[test]
    fn test_no_2point_swing() {
        // Alice vouched and flagged Bob (contradiction)
        // all_vouchers: 2 (Alice, Carol)
        // all_flaggers: 1 (Alice)
        // voucher_flaggers: 1 (Alice in both sets)
        // effective_vouches: 2 - 1 = 1
        // regular_flags: 1 - 1 = 0
        // standing: 1 - 0 = 1

        // Original: standing = 2 (2 vouches, 0 flags)
        // After Alice flags: standing = 1 (1 point drop, not 2)

        let trigger = EjectionTrigger::should_eject(2, 1, 1, 2);

        // Ejected due to threshold (effective_vouches < 2), not negative standing
        match trigger {
            Some(EjectionTrigger::BelowThreshold {
                effective_vouches,
                min_threshold,
            }) => {
                assert_eq!(effective_vouches, 1);
                assert_eq!(min_threshold, 2);
            }
            _ => panic!("Expected BelowThreshold trigger"),
        }
    }
}
