//! Ejection Protocol
//!
//! Per vetting-protocols.bead and security-constraints.bead:
//! - Two independent triggers: Standing < 0 OR Effective_Vouches < 2
//! - NO GRACE PERIODS - immediate execution
//! - Signal API retry with logarithmic backoff (GAP-06)
//! - Invariant: signal_state.members ⊆ freenet_state.members
//!
//! Ejection process:
//! 1. Remove from Signal group (with retry)
//! 2. Send PM to ejected member (using hash, not cleartext)
//! 3. Announce to group (using hash, not name)
//! 4. Update Freenet: move to `ejected` set

use crate::freenet::contract::MemberHash;
use crate::freenet::traits::{ContractDelta, ContractHash, FreenetClient};
use crate::freenet::trust_contract::{StateDelta, TrustNetworkState};
use crate::signal::retry::{is_signal_error_retryable, retry_with_backoff};
use crate::signal::traits::{GroupId, ServiceId, SignalClient};
use std::sync::Arc;

/// Ejection errors.
#[derive(Debug, thiserror::Error)]
pub enum EjectionError {
    #[error("Signal error: {0}")]
    SignalError(String),

    #[error("Freenet error: {0}")]
    FreenetError(String),

    #[error("Member not found in mapping: {0:?}")]
    MemberNotMapped(MemberHash),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Ejection result type.
pub type EjectionResult<T> = Result<T, EjectionError>;

/// Eject a member from the network.
///
/// This function handles the complete ejection process:
/// 1. Remove from Signal group (with retry for transient failures)
/// 2. Send PM to ejected member (using hash)
/// 3. Announce to group (using hash, not name)
/// 4. Update Freenet state (move to ejected set)
///
/// Per GAP-06: Signal API calls retry with logarithmic backoff (2^n seconds, capped at 1 hour).
///
/// # Arguments
///
/// * `member` - The member to eject
/// * `member_service_id` - The Signal ServiceId for this member
/// * `signal` - Signal client
/// * `freenet` - Freenet client
/// * `contract` - Trust network contract hash
/// * `group_id` - Signal group ID
///
/// # Returns
///
/// Ok(()) if ejection successful, Err otherwise.
pub async fn eject_member<S, F>(
    member: &MemberHash,
    member_service_id: &ServiceId,
    signal: &S,
    freenet: Arc<F>,
    contract: &ContractHash,
    group_id: &GroupId,
) -> EjectionResult<()>
where
    S: SignalClient,
    F: FreenetClient,
{
    // Step 1: Remove from Signal group (with retry)
    let service_id_clone = member_service_id.clone();
    let group_id_clone = group_id.clone();
    let signal_clone = signal.clone();

    retry_with_backoff(
        || {
            let signal = signal_clone.clone();
            let group = group_id_clone.clone();
            let service = service_id_clone.clone();
            async move { signal.remove_group_member(&group, &service).await }
        },
        is_signal_error_retryable,
    )
    .await
    .map_err(|e| EjectionError::SignalError(e.to_string()))?;

    // Step 2: Send PM to ejected member (using hash)
    let pm_message = format!(
        "You have been ejected from the group.\n\
         \n\
         Reason: Failed to maintain required standing.\n\
         \n\
         Your member hash: {}\n\
         \n\
         You may re-enter if you receive new vouches meeting the admission threshold.",
        format_member_hash(member)
    );

    // Send PM with retry
    retry_with_backoff(
        || {
            let signal = signal_clone.clone();
            let service = service_id_clone.clone();
            let msg = pm_message.clone();
            async move { signal.send_message(&service, &msg).await }
        },
        is_signal_error_retryable,
    )
    .await
    .map_err(|e| EjectionError::SignalError(format!("Failed to send PM: {}", e)))?;

    // Step 3: Announce to group (using hash, not name)
    let announcement = format!(
        "Member {} has been ejected for failing to maintain required standing.",
        format_member_hash(member)
    );

    retry_with_backoff(
        || {
            let signal = signal_clone.clone();
            let group = group_id_clone.clone();
            let msg = announcement.clone();
            async move { signal.send_group_message(&group, &msg).await }
        },
        is_signal_error_retryable,
    )
    .await
    .map_err(|e| EjectionError::SignalError(format!("Failed to send announcement: {}", e)))?;

    // Step 4: Update Freenet state (move to ejected set)
    update_freenet_ejection(freenet, contract, member).await?;

    Ok(())
}

/// Update Freenet state to reflect ejection.
///
/// Creates a StateDelta that:
/// - Removes member from active members
/// - Adds member to ejected set
///
/// Per vetting-protocols.bead: "Ejected members can re-enter with new vouches"
async fn update_freenet_ejection<F>(
    freenet: Arc<F>,
    contract: &ContractHash,
    member: &MemberHash,
) -> EjectionResult<()>
where
    F: FreenetClient,
{
    // Create delta to move member to ejected set
    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![*member],
        vouches_added: vec![],
        vouches_removed: vec![],
        flags_added: vec![],
        flags_removed: vec![],
        config_update: None,
        proposals_created: vec![],
        proposals_checked: vec![],
        proposals_with_results: vec![],
        audit_entries_added: vec![],
        gap11_announcement_sent: false,
    };

    // Serialize delta to CBOR
    let delta_bytes = crate::serialization::to_cbor(&delta)
        .map_err(|e| EjectionError::SerializationError(e.to_string()))?;

    // Apply delta to Freenet contract
    let contract_delta = ContractDelta { data: delta_bytes };

    freenet
        .apply_delta(contract, &contract_delta)
        .await
        .map_err(|e| EjectionError::FreenetError(e.to_string()))?;

    Ok(())
}

/// Format a member hash for display (truncated for readability).
///
/// Shows first 8 hex chars + "..." to avoid overwhelming messages.
fn format_member_hash(member: &MemberHash) -> String {
    let hash_hex = hex::encode(member.as_bytes());
    if hash_hex.len() > 16 {
        format!("{}...{}", &hash_hex[..8], &hash_hex[hash_hex.len() - 8..])
    } else {
        hash_hex
    }
}

/// Check if a member should be ejected based on standing and vouches.
///
/// Per security-constraints.bead, two independent triggers:
/// 1. Standing < 0
/// 2. Effective_Vouches < min_vouch_threshold
pub fn should_eject(state: &TrustNetworkState, member: &MemberHash) -> bool {
    // Only check active members
    if !state.members.contains(member) {
        return false;
    }

    // Calculate metrics
    let vouchers = state.vouchers_for(member);
    let flaggers = state.flaggers_for(member);

    // Find voucher-flaggers (contradictory members)
    let voucher_set: std::collections::HashSet<_> = vouchers.into_iter().collect();
    let flagger_set: std::collections::HashSet<_> = flaggers.into_iter().collect();
    let voucher_flaggers: std::collections::HashSet<_> =
        voucher_set.intersection(&flagger_set).copied().collect();

    // Calculate effective vouches
    let effective_vouches = (voucher_set.len() - voucher_flaggers.len()) as u32;

    // Calculate standing
    let regular_flags = (flagger_set.len() - voucher_flaggers.len()) as i32;
    let standing = effective_vouches as i32 - regular_flags;

    // Trigger 1: Effective vouches below threshold
    if effective_vouches < state.config.min_vouches {
        return true;
    }

    // Trigger 2: Standing below zero
    if standing < 0 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::mock::MockFreenetClient;
    use crate::signal::traits::SignalError;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock Signal Client for testing
    #[derive(Clone)]
    struct MockSignalClient {
        removed_members: Arc<Mutex<Vec<ServiceId>>>,
        sent_messages: Arc<Mutex<Vec<(ServiceId, String)>>>,
        group_messages: Arc<Mutex<Vec<(GroupId, String)>>>,
        service_id: ServiceId,
        fail_remove: Arc<Mutex<bool>>,
    }

    impl MockSignalClient {
        fn new() -> Self {
            Self {
                removed_members: Arc::new(Mutex::new(Vec::new())),
                sent_messages: Arc::new(Mutex::new(Vec::new())),
                group_messages: Arc::new(Mutex::new(Vec::new())),
                service_id: ServiceId("bot".to_string()),
                fail_remove: Arc::new(Mutex::new(false)),
            }
        }

        async fn get_removed_members(&self) -> Vec<ServiceId> {
            self.removed_members.lock().await.clone()
        }

        async fn get_sent_messages(&self) -> Vec<(ServiceId, String)> {
            self.sent_messages.lock().await.clone()
        }

        async fn get_group_messages(&self) -> Vec<(GroupId, String)> {
            self.group_messages.lock().await.clone()
        }

        #[allow(dead_code)]
        fn set_fail_remove(&self, fail: bool) {
            *self.fail_remove.blocking_lock() = fail;
        }
    }

    #[async_trait]
    impl SignalClient for MockSignalClient {
        async fn send_message(&self, recipient: &ServiceId, text: &str) -> Result<(), SignalError> {
            self.sent_messages
                .lock()
                .await
                .push((recipient.clone(), text.to_string()));
            Ok(())
        }

        async fn send_group_message(&self, group: &GroupId, text: &str) -> Result<(), SignalError> {
            self.group_messages
                .lock()
                .await
                .push((group.clone(), text.to_string()));
            Ok(())
        }

        async fn create_group(&self, _name: &str) -> Result<GroupId, SignalError> {
            Ok(GroupId(vec![1, 2, 3]))
        }

        async fn add_group_member(
            &self,
            _group: &GroupId,
            _member: &ServiceId,
        ) -> Result<(), SignalError> {
            Ok(())
        }

        async fn remove_group_member(
            &self,
            _group: &GroupId,
            member: &ServiceId,
        ) -> Result<(), SignalError> {
            if *self.fail_remove.lock().await {
                return Err(SignalError::Network("transient failure".to_string()));
            }
            self.removed_members.lock().await.push(member.clone());
            Ok(())
        }

        async fn create_poll(
            &self,
            _group: &GroupId,
            _poll: &crate::signal::traits::Poll,
        ) -> Result<u64, SignalError> {
            Ok(1)
        }

        async fn terminate_poll(
            &self,
            _group: &crate::signal::traits::GroupId,
            _poll_timestamp: u64,
        ) -> Result<(), SignalError> {
            Ok(())
        }

        async fn receive_messages(
            &self,
        ) -> Result<Vec<crate::signal::traits::Message>, SignalError> {
            Ok(Vec::new())
        }

        fn service_id(&self) -> &ServiceId {
            &self.service_id
        }
    }

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    fn test_service_id(id: u8) -> ServiceId {
        ServiceId(format!("user_{}", id))
    }

    #[tokio::test]
    async fn test_eject_member_complete_flow() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        // Setup: Put initial state in Freenet
        let initial_state = TrustNetworkState::new();
        let state_bytes = initial_state.to_bytes().unwrap();
        freenet.put_state(
            contract,
            crate::freenet::traits::ContractState { data: state_bytes },
        );

        let member = test_member(1);
        let service_id = test_service_id(1);

        // Eject member
        eject_member(
            &member,
            &service_id,
            &signal,
            freenet.clone(),
            &contract,
            &group_id,
        )
        .await
        .unwrap();

        // Verify Signal removal
        let removed = signal.get_removed_members().await;
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], service_id);

        // Verify PM sent
        let messages = signal.get_sent_messages().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, service_id);
        assert!(messages[0].1.contains("ejected"));

        // Verify group announcement
        let announcements = signal.get_group_messages().await;
        assert_eq!(announcements.len(), 1);
        assert_eq!(announcements[0].0, group_id);
        assert!(announcements[0].1.contains("ejected"));
    }

    #[tokio::test]
    async fn test_should_eject_trigger_1_low_vouches() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;

        let member = test_member(1);
        let voucher = test_member(2);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher].into_iter().collect());

        // Only 1 vouch, threshold is 2 -> should eject
        assert!(should_eject(&state, &member));
    }

    #[tokio::test]
    async fn test_should_eject_trigger_2_negative_standing() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;

        let member = test_member(1);
        let voucher1 = test_member(2);
        let voucher2 = test_member(3);
        let flagger1 = test_member(4);
        let flagger2 = test_member(5);
        let flagger3 = test_member(6);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher1, voucher2].into_iter().collect());
        state
            .flags
            .insert(member, [flagger1, flagger2, flagger3].into_iter().collect());

        // Standing = 2 - 3 = -1 -> should eject
        assert!(should_eject(&state, &member));
    }

    #[tokio::test]
    async fn test_should_not_eject_good_standing() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;

        let member = test_member(1);
        let voucher1 = test_member(2);
        let voucher2 = test_member(3);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher1, voucher2].into_iter().collect());

        // 2 vouches, 0 flags -> good standing
        assert!(!should_eject(&state, &member));
    }

    #[tokio::test]
    async fn test_format_member_hash() {
        let member = MemberHash::from_bytes(&[0x42u8; 32]);
        let formatted = format_member_hash(&member);

        // Should be truncated with ellipsis
        assert!(formatted.contains("..."));
        assert!(formatted.len() < 64); // Less than full hash
    }

    #[tokio::test]
    async fn test_should_eject_with_voucher_flaggers() {
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;

        let bob = test_member(1);
        let alice = test_member(2);
        let carol = test_member(3);

        state.members.insert(bob);
        // Bob has 2 vouches from Alice and Carol
        state
            .vouches
            .insert(bob, [alice, carol].into_iter().collect());
        // Alice flags Bob (voucher-flagger)
        state.flags.insert(bob, [alice].into_iter().collect());

        // Effective vouches = 2 - 1 = 1 (below threshold of 2)
        // Should eject
        assert!(should_eject(&state, &bob));
    }

    #[tokio::test]
    async fn test_should_not_eject_non_member() {
        let state = TrustNetworkState::new();
        let member = test_member(1);

        // Member not in members set -> should not eject
        assert!(!should_eject(&state, &member));
    }
}
