//! Continuous health monitoring for trust network standing.
//!
//! Per vetting-protocols.bead:
//! - Real-time state stream (NOT polling)
//! - Check both ejection triggers on every state change
//! - Immediate action when threshold violated
//!
//! Per security-constraints.bead:
//! - Trigger 1: effective_vouches < min_vouch_threshold
//! - Trigger 2: standing < 0
//!
//! Standing calculation:
//! - Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
//! - Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
//! - Standing = Effective_Vouches - Regular_Flags

use crate::freenet::contract::MemberHash;
use crate::freenet::traits::{ContractHash, FreenetClient};
use crate::freenet::trust_contract::TrustNetworkState;
use crate::signal::traits::{GroupId, ServiceId, SignalClient};
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Health monitor for continuous standing checks.
///
/// Per freenet-integration.bead: "Real-Time Stream (REQUIRED - never poll)"
pub struct HealthMonitor<F, S>
where
    F: FreenetClient,
    S: SignalClient,
{
    freenet: Arc<F>,
    signal: S,
    contract: ContractHash,
    group_id: GroupId,
    /// Mapping from MemberHash to Signal ServiceId (HMAC-masked, never cleartext)
    member_mapping: Arc<RwLock<HashMap<MemberHash, ServiceId>>>,
}

impl<F, S> HealthMonitor<F, S>
where
    F: FreenetClient + 'static,
    S: SignalClient + 'static,
{
    /// Create a new health monitor.
    pub fn new(freenet: Arc<F>, signal: S, contract: ContractHash, group_id: GroupId) -> Self {
        Self {
            freenet,
            signal,
            contract,
            group_id,
            member_mapping: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a member mapping (MemberHash -> ServiceId).
    ///
    /// This mapping is HMAC-masked and never stores cleartext Signal IDs.
    pub async fn register_member(&self, member_hash: MemberHash, service_id: ServiceId) {
        let mut mapping = self.member_mapping.write().await;
        mapping.insert(member_hash, service_id);
    }

    /// Unregister a member mapping.
    pub async fn unregister_member(&self, member_hash: &MemberHash) {
        let mut mapping = self.member_mapping.write().await;
        mapping.remove(member_hash);
    }

    /// Start monitoring (runs until cancelled).
    ///
    /// Per vetting-protocols.bead:
    /// - Use real-time Freenet state stream (react to StateChange events)
    /// - Immediate ejection when either trigger activated
    pub async fn run(&self) -> Result<(), MonitorError> {
        // Subscribe to state stream
        let mut stream = self
            .freenet
            .subscribe(&self.contract)
            .await
            .map_err(|e| MonitorError::SubscriptionFailed(e.to_string()))?;

        // Monitor state changes
        loop {
            tokio::select! {
                Some(change) = stream.next() => {
                    // Get current state
                    let state_bytes = self.freenet
                        .get_state(&change.contract)
                        .await
                        .map_err(|e| MonitorError::StateReadFailed(e.to_string()))?;

                    let state = TrustNetworkState::from_bytes(&state_bytes.data)
                        .map_err(|e| MonitorError::DeserializationFailed(e.to_string()))?;

                    // Check all members' standing
                    self.check_all_members(&state).await?;
                }
                else => {
                    // Stream ended
                    break;
                }
            }
        }

        Ok(())
    }

    /// Check standing for all members and trigger ejection if needed.
    async fn check_all_members(&self, state: &TrustNetworkState) -> Result<(), MonitorError> {
        let min_vouches = state.config.min_vouches;

        for member in &state.members {
            if self.should_eject(member, state, min_vouches).await? {
                self.eject_member(member).await?;
            }
        }

        Ok(())
    }

    /// Check if a member should be ejected.
    ///
    /// Per security-constraints.bead, two independent triggers:
    /// 1. Effective_Vouches < min_vouch_threshold
    /// 2. Standing < 0
    async fn should_eject(
        &self,
        member: &MemberHash,
        state: &TrustNetworkState,
        min_vouches: u32,
    ) -> Result<bool, MonitorError> {
        let metrics = self.calculate_standing(member, state);

        // Trigger 1: Effective vouches below threshold
        if metrics.effective_vouches < min_vouches {
            return Ok(true);
        }

        // Trigger 2: Standing below zero
        if metrics.standing < 0 {
            return Ok(true);
        }

        Ok(false)
    }

    /// Calculate standing metrics for a member.
    ///
    /// Per security-constraints.bead:
    /// - Effective_Vouches = |All_Vouchers| - |Voucher_Flaggers|
    /// - Regular_Flags = |All_Flaggers| - |Voucher_Flaggers|
    /// - Standing = Effective_Vouches - Regular_Flags
    /// - Voucher_Flaggers = members who both vouch AND flag the same person
    fn calculate_standing(
        &self,
        member: &MemberHash,
        state: &TrustNetworkState,
    ) -> StandingMetrics {
        // Get all vouchers and flaggers
        let all_vouchers: HashSet<MemberHash> = state.vouchers_for(member).into_iter().collect();

        let all_flaggers: HashSet<MemberHash> = state.flaggers_for(member).into_iter().collect();

        // Calculate voucher-flaggers (contradictory members)
        let voucher_flaggers: HashSet<MemberHash> =
            all_vouchers.intersection(&all_flaggers).copied().collect();

        // Calculate metrics
        let effective_vouches = (all_vouchers.len() - voucher_flaggers.len()) as u32;
        let regular_flags = (all_flaggers.len() - voucher_flaggers.len()) as u32;
        let standing = effective_vouches as i32 - regular_flags as i32;

        StandingMetrics {
            effective_vouches,
            regular_flags,
            standing,
        }
    }

    /// Eject a member from the Signal group.
    ///
    /// Per security-constraints.bead:
    /// "Both triggers result in IMMEDIATE ejection (no delay, no notification beforehand)"
    async fn eject_member(&self, member: &MemberHash) -> Result<(), MonitorError> {
        // Get Signal ServiceId for this member
        let mapping = self.member_mapping.read().await;
        let service_id = mapping
            .get(member)
            .ok_or(MonitorError::MemberNotMapped(*member))?;

        // Remove from Signal group
        self.signal
            .remove_group_member(&self.group_id, service_id)
            .await
            .map_err(|e| MonitorError::SignalError(e.to_string()))?;

        Ok(())
    }
}

/// Standing metrics for a member.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StandingMetrics {
    /// Effective vouches (vouches minus contradictory members)
    effective_vouches: u32,
    /// Regular flags (flags minus contradictory members)
    regular_flags: u32,
    /// Standing = effective_vouches - regular_flags
    standing: i32,
}

/// Health monitor errors.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("State read failed: {0}")]
    StateReadFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Signal error: {0}")]
    SignalError(String),

    #[error("Member not mapped: {0:?}")]
    MemberNotMapped(MemberHash),
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
        service_id: ServiceId,
    }

    impl MockSignalClient {
        fn new() -> Self {
            Self {
                removed_members: Arc::new(Mutex::new(Vec::new())),
                service_id: ServiceId("bot".to_string()),
            }
        }

        async fn get_removed_members(&self) -> Vec<ServiceId> {
            self.removed_members.lock().await.clone()
        }
    }

    #[async_trait]
    impl SignalClient for MockSignalClient {
        async fn send_message(
            &self,
            _recipient: &ServiceId,
            _text: &str,
        ) -> Result<(), SignalError> {
            Ok(())
        }

        async fn send_group_message(
            &self,
            _group: &GroupId,
            _text: &str,
        ) -> Result<(), SignalError> {
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
    async fn test_standing_calculation_basic() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Create state with member having 2 vouches, 0 flags
        let mut state = TrustNetworkState::new();
        let member = test_member(1);
        let voucher1 = test_member(2);
        let voucher2 = test_member(3);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher1, voucher2].into_iter().collect());

        let metrics = monitor.calculate_standing(&member, &state);

        assert_eq!(metrics.effective_vouches, 2);
        assert_eq!(metrics.regular_flags, 0);
        assert_eq!(metrics.standing, 2);
    }

    #[tokio::test]
    async fn test_standing_calculation_with_voucher_flagger() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Create state: Bob has 2 vouches (Alice, Carol), Alice flags Bob
        let mut state = TrustNetworkState::new();
        let bob = test_member(1);
        let alice = test_member(2);
        let carol = test_member(3);

        state.members.insert(bob);
        state
            .vouches
            .insert(bob, [alice, carol].into_iter().collect());
        state.flags.insert(bob, [alice].into_iter().collect());

        let metrics = monitor.calculate_standing(&bob, &state);

        // Per security-constraints.bead:
        // Voucher_Flaggers = {Alice}
        // Effective_Vouches = 2 - 1 = 1
        // Regular_Flags = 1 - 1 = 0
        // Standing = 1 - 0 = 1
        assert_eq!(metrics.effective_vouches, 1);
        assert_eq!(metrics.regular_flags, 0);
        assert_eq!(metrics.standing, 1);
    }

    #[tokio::test]
    async fn test_should_eject_trigger_1_effective_vouches() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Member with only 1 effective vouch (below threshold of 2)
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;
        let member = test_member(1);
        let voucher = test_member(2);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher].into_iter().collect());

        let should_eject = monitor
            .should_eject(&member, &state, state.config.min_vouches)
            .await
            .unwrap();
        assert!(should_eject);
    }

    #[tokio::test]
    async fn test_should_eject_trigger_2_negative_standing() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Member with 2 vouches but 3 flags -> standing = 2 - 3 = -1
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

        let should_eject = monitor
            .should_eject(&member, &state, state.config.min_vouches)
            .await
            .unwrap();
        assert!(should_eject);
    }

    #[tokio::test]
    async fn test_should_not_eject_good_standing() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Member with 2 vouches, 0 flags -> good standing
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;
        let member = test_member(1);
        let voucher1 = test_member(2);
        let voucher2 = test_member(3);

        state.members.insert(member);
        state
            .vouches
            .insert(member, [voucher1, voucher2].into_iter().collect());

        let should_eject = monitor
            .should_eject(&member, &state, state.config.min_vouches)
            .await
            .unwrap();
        assert!(!should_eject);
    }

    #[tokio::test]
    async fn test_eject_member_removes_from_signal() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet.clone(), signal.clone(), contract, group_id);

        // Register member
        let member = test_member(1);
        let service_id = test_service_id(1);
        monitor.register_member(member, service_id.clone()).await;

        // Eject member
        monitor.eject_member(&member).await.unwrap();

        // Verify Signal removal
        let removed = signal.get_removed_members().await;
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], service_id);
    }

    #[tokio::test]
    async fn test_eject_unmapped_member_fails() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet, signal, contract, group_id);

        // Try to eject member without mapping
        let member = test_member(1);
        let result = monitor.eject_member(&member).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MonitorError::MemberNotMapped(_)
        ));
    }

    #[tokio::test]
    async fn test_check_all_members_ejects_violators() {
        let freenet = Arc::new(MockFreenetClient::new());
        let signal = MockSignalClient::new();
        let contract = ContractHash::from_bytes(&[0u8; 32]);
        let group_id = GroupId(vec![1, 2, 3]);

        let monitor = HealthMonitor::new(freenet.clone(), signal.clone(), contract, group_id);

        // Create state with 3 members:
        // - Member 1: Good (2 vouches)
        // - Member 2: Bad (1 vouch, below threshold)
        // - Member 3: Bad (2 vouches, 3 flags, standing = -1)
        let mut state = TrustNetworkState::new();
        state.config.min_vouches = 2;

        let member1 = test_member(1);
        let member2 = test_member(2);
        let member3 = test_member(3);

        state.members.insert(member1);
        state.members.insert(member2);
        state.members.insert(member3);

        // Member 1: 2 vouches (good)
        state.vouches.insert(
            member1,
            [test_member(10), test_member(11)].into_iter().collect(),
        );

        // Member 2: 1 vouch (below threshold)
        state
            .vouches
            .insert(member2, [test_member(12)].into_iter().collect());

        // Member 3: 2 vouches, 3 flags (negative standing)
        state.vouches.insert(
            member3,
            [test_member(13), test_member(14)].into_iter().collect(),
        );
        state.flags.insert(
            member3,
            [test_member(15), test_member(16), test_member(17)]
                .into_iter()
                .collect(),
        );

        // Register all members
        monitor.register_member(member1, test_service_id(1)).await;
        monitor.register_member(member2, test_service_id(2)).await;
        monitor.register_member(member3, test_service_id(3)).await;

        // Check all members
        monitor.check_all_members(&state).await.unwrap();

        // Verify only member 2 and 3 were ejected
        let removed = signal.get_removed_members().await;
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&test_service_id(2)));
        assert!(removed.contains(&test_service_id(3)));
    }
}
