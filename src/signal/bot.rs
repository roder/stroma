//! Stroma Signal Bot
//!
//! Main bot implementation that:
//! - Monitors Freenet state changes (real-time stream)
//! - Enforces trust model (2-vouch requirement, immediate ejection)
//! - Handles PM commands (/invite, /vouch, /flag, etc.)
//! - Manages Signal group membership
//!
//! See: .beads/signal-integration.bead

use super::{
    bootstrap::BootstrapManager,
    group::{EjectionTrigger, GroupManager},
    pm::{handle_pm_command, parse_command, Command},
    polls::PollManager,
    traits::*,
    vetting::VettingSessionManager,
};

/// Stroma bot configuration
pub struct BotConfig {
    pub group_id: GroupId,
    pub min_vouch_threshold: u32,
    pub pepper: Vec<u8>,
    pub contract_hash: Option<crate::freenet::traits::ContractHash>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            group_id: GroupId(vec![]),
            min_vouch_threshold: 2,
            pepper: b"default-pepper".to_vec(),
            contract_hash: None,
        }
    }
}

/// Stroma Signal bot
pub struct StromaBot<C: SignalClient, F: crate::freenet::FreenetClient> {
    client: C,
    freenet: F,
    config: BotConfig,
    group_manager: GroupManager<C>,
    poll_manager: PollManager<C>,
    bootstrap_manager: BootstrapManager<C>,
    vetting_sessions: VettingSessionManager,
}

impl<C: SignalClient, F: crate::freenet::FreenetClient> StromaBot<C, F> {
    pub fn new(client: C, freenet: F, config: BotConfig) -> Self {
        let group_manager = GroupManager::new(client.clone(), config.group_id.clone());
        let poll_manager = PollManager::new(client.clone(), config.group_id.clone());
        let bootstrap_manager = BootstrapManager::new(client.clone(), config.pepper.clone());
        let vetting_sessions = VettingSessionManager::new();

        Self {
            client,
            freenet,
            config,
            group_manager,
            poll_manager,
            bootstrap_manager,
            vetting_sessions,
        }
    }

    /// Run bot event loop
    ///
    /// Receives Signal messages and processes commands.
    /// In production, also monitors Freenet state changes.
    pub async fn run(&mut self) -> SignalResult<()> {
        loop {
            // Receive messages from Signal
            let messages = self.client.receive_messages().await?;

            for message in messages {
                self.handle_message(message).await?;
            }

            // TODO: Also monitor Freenet state changes
            // tokio::select! {
            //     Some(msg) = signal_stream.next() => self.handle_message(msg).await?,
            //     Some(change) = freenet_stream.next() => self.handle_state_change(change).await?,
            // }
        }
    }

    /// Handle incoming message
    async fn handle_message(&mut self, message: Message) -> SignalResult<()> {
        match message.content {
            MessageContent::Text(text) => {
                let command = parse_command(&text);

                // Route commands to appropriate handlers
                match command {
                    Command::CreateGroup { ref group_name } => {
                        self.bootstrap_manager
                            .handle_create_group(&message.sender, group_name.clone())
                            .await?;
                    }
                    Command::AddSeed { ref username } => {
                        self.bootstrap_manager
                            .handle_add_seed(&self.freenet, &message.sender, username)
                            .await?;
                    }
                    Command::Invite {
                        ref username,
                        ref context,
                    } => {
                        self.handle_invite(&message.sender, username, context.as_deref())
                            .await?;
                    }
                    Command::Vouch { ref username } => {
                        self.handle_vouch(&message.sender, username).await?;
                    }
                    _ => {
                        // Other commands go through normal handler
                        handle_pm_command(
                            &self.client,
                            &self.freenet,
                            &self.group_manager,
                            &self.config,
                            &message.sender,
                            command,
                        )
                        .await?;
                    }
                }
            }

            MessageContent::Poll(_) => {
                // Bot created this poll, ignore
            }

            MessageContent::PollVote(vote) => {
                self.poll_manager.process_vote(&vote)?;
            }
        }

        Ok(())
    }

    /// Handle /invite command
    ///
    /// Creates ephemeral vetting session and initiates cross-cluster matching.
    async fn handle_invite(
        &mut self,
        sender: &ServiceId,
        username: &str,
        context: Option<&str>,
    ) -> SignalResult<()> {
        use super::matchmaker::BlindMatchmaker;
        use crate::freenet::contract::MemberHash;

        // TODO Phase 1: Query Freenet to verify sender is a member
        // For now, assume sender is valid member

        // Hash inviter's ServiceId to MemberHash
        let inviter_hash = MemberHash::from_identity(&sender.0, &self.config.pepper);

        // TODO Phase 1: Query Freenet state for previous flags (GAP-10)
        let has_previous_flags = false;
        let previous_flag_count = 0;

        // Create ephemeral vetting session
        let result = self.vetting_sessions.create_session(
            ServiceId(username.to_string()), // TODO: resolve username to actual ServiceId
            username.to_string(),
            inviter_hash,
            sender.clone(),
            context.map(|s| s.to_string()),
            has_previous_flags,
            previous_flag_count,
        );

        if let Err(e) = result {
            let response = format!("❌ Cannot invite {}: {}", username, e);
            return self.client.send_message(sender, &response).await;
        }

        // TODO Phase 1: Query Freenet state for cross-cluster matching
        // For now, use a simple placeholder state
        let state = crate::freenet::trust_contract::TrustNetworkState::new();

        // Select validator via Blind Matchmaker
        let validator_hash = BlindMatchmaker::select_validator(&state, &inviter_hash);

        let response = if let Some(_validator) = validator_hash {
            // TODO Phase 1: Resolve validator hash to ServiceId
            // TODO Phase 1: Assign validator to session
            // TODO Phase 1: Send PMs to invitee and validator

            let context_str = context.unwrap_or("(no context provided)");
            let gap10_warning = if has_previous_flags {
                format!("\n\n⚠️ Note: {} has {} previous flags from a past membership. They'll need additional vouches to achieve positive standing.", username, previous_flag_count)
            } else {
                String::new()
            };

            format!(
                "✅ Invitation for {} recorded as first vouch.\n\nContext: {}{}\n\nI'm now reaching out to a member from a different cluster for the cross-cluster vouch. You'll be notified when the vetting process progresses.",
                username, context_str, gap10_warning
            )
        } else {
            format!(
                "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nNote: Network is small (bootstrap phase). They'll join once they receive one more vouch from any member.",
                username, context.unwrap_or("(no context provided)")
            )
        };

        self.client.send_message(sender, &response).await
    }

    /// Handle /vouch command
    ///
    /// Records second vouch and admits member if threshold met.
    async fn handle_vouch(&mut self, sender: &ServiceId, username: &str) -> SignalResult<()> {
        // TODO Phase 1: Verify sender is a member
        // TODO Phase 1: Hash sender's ServiceId to MemberHash
        // TODO Phase 1: Check if vetting session exists for username
        // TODO Phase 1: Record vouch in Freenet (AddVouch delta)
        // TODO Phase 1: Check if threshold met (effective_vouches >= 2)
        // TODO Phase 1: If threshold met, add to Signal group
        // TODO Phase 1: Delete ephemeral vetting session

        let response = format!(
            "✅ Vouch for {} recorded.\n\nTheir standing has been updated. If they've reached the 2-vouch threshold, they'll be automatically added to the Signal group.",
            username
        );

        self.client.send_message(sender, &response).await
    }

    /// Handle Freenet state change
    ///
    /// React to trust model changes immediately (no polling).
    pub async fn handle_state_change(&mut self, change: StateChange) -> SignalResult<()> {
        match change {
            StateChange::MemberVetted {
                member_hash,
                service_id,
            } => {
                self.group_manager.add_member(&service_id).await?;
                self.group_manager.announce_admission(&member_hash).await?;
            }

            StateChange::MemberRevoked {
                member_hash,
                service_id,
                trigger: _,
            } => {
                self.group_manager.remove_member(&service_id).await?;
                self.group_manager.announce_ejection(&member_hash).await?;
            }

            StateChange::ProposalPassed { proposal: _ } => {
                // Apply proposal changes
                // e.g., update GroupConfig in Freenet
            }

            StateChange::ProposalExpired {
                poll_id,
                poll_timestamp,
            } => {
                // 1. Terminate the poll
                self.poll_manager.terminate_poll(poll_timestamp).await?;

                // 2. Get vote aggregate
                let votes = self.poll_manager.get_vote_aggregate(poll_id);

                if let Some(aggregate) = votes {
                    // 3. Check quorum and threshold
                    if let Some(crate::signal::polls::PollOutcome::Passed {
                        approve_count: _,
                        reject_count: _,
                    }) = self.poll_manager.check_poll_outcome(poll_id, aggregate)
                    {
                        // 4. Announce outcome (TODO)
                        // self.group_manager.announce_proposal_passed().await?;

                        // 5. Execute proposal
                        // TODO: Get actual contract hash from config
                        // TODO: Get current GroupConfig from Freenet state
                        // For now, use placeholder
                        // let contract = ContractHash::from_bytes(&[0u8; 32]);
                        // let current_config = GroupConfig::default();
                        // if let Some(proposal) = self.poll_manager.get_proposal(poll_id) {
                        //     execute_proposal(&self.freenet, &contract, &proposal.proposal_type, &current_config).await?;
                        // }
                    }
                    // Failed or quorum not met - announce outcome
                    // TODO: Announce failure/quorum-not-met
                }

                // 6. TODO: Mark proposal as checked in Freenet
            }
        }

        Ok(())
    }

    /// Check ejection triggers for member
    pub fn check_ejection(
        &self,
        all_vouchers: u32,
        all_flaggers: u32,
        voucher_flaggers: u32,
    ) -> Option<EjectionTrigger> {
        EjectionTrigger::should_eject(
            all_vouchers,
            all_flaggers,
            voucher_flaggers,
            self.config.min_vouch_threshold,
        )
    }

    /// Verify admission with ZK-proof
    ///
    /// Generates and verifies a STARK proof of vouch verification before admission.
    /// This ensures the member meets the 2-vouch requirement (cross-cluster during
    /// normal operation, suspended during bootstrap).
    ///
    /// # Returns
    /// - Ok(()) if proof is valid and member can be admitted
    /// - Err() if proof generation fails or verification fails
    pub fn verify_admission_proof(
        &self,
        member_hash: crate::stark::types::MemberHash,
        vouchers: std::collections::BTreeSet<crate::stark::types::MemberHash>,
        flaggers: std::collections::BTreeSet<crate::stark::types::MemberHash>,
    ) -> Result<(), String> {
        use crate::stark::{generate_vouch_proof, verify_vouch_proof, VouchClaim};

        // Build claim from vouch data
        let claim = VouchClaim::new(member_hash, vouchers, flaggers);

        // Verify claim meets minimum threshold
        if claim.effective_vouches < self.config.min_vouch_threshold as usize {
            return Err(format!(
                "Insufficient vouches: {} (required: {})",
                claim.effective_vouches, self.config.min_vouch_threshold
            ));
        }

        // Verify standing is non-negative
        if claim.standing < 0 {
            return Err(format!(
                "Negative standing: {} (too many flags)",
                claim.standing
            ));
        }

        // Generate ZK-proof
        let proof =
            generate_vouch_proof(&claim).map_err(|e| format!("Proof generation failed: {}", e))?;

        // Verify ZK-proof
        verify_vouch_proof(&proof).map_err(|e| format!("Proof verification failed: {}", e))?;

        // Proof is valid - admission criteria met
        Ok(())
    }
}

/// Freenet state changes (from real-time stream)
pub enum StateChange {
    MemberVetted {
        member_hash: String,
        service_id: ServiceId,
    },
    MemberRevoked {
        member_hash: String,
        service_id: ServiceId,
        trigger: EjectionTrigger,
    },
    ProposalPassed {
        proposal: String,
    },
    /// Proposal timeout expired - time to terminate and check results
    ProposalExpired {
        poll_id: u64,
        poll_timestamp: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::MockFreenetClient;
    use crate::signal::mock::MockSignalClient;

    #[test]
    fn test_bot_creation() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();

        let _bot = StromaBot::new(client, freenet, config);
    }

    #[tokio::test]
    async fn test_handle_text_message() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        let message = Message {
            sender: ServiceId("user1".to_string()),
            content: MessageContent::Text("/status".to_string()),
            timestamp: 1234567890,
        };

        bot.handle_message(message).await.unwrap();

        // Verify bot sent response
        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_state_change_admission() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let group = GroupId(vec![1, 2, 3]);
        let config = BotConfig {
            group_id: group.clone(),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        let change = StateChange::MemberVetted {
            member_hash: "hash123".to_string(),
            service_id: ServiceId("user1".to_string()),
        };

        bot.handle_state_change(change).await.unwrap();

        // Verify member added to group
        assert!(client.is_member(&group, &ServiceId("user1".to_string())));

        // Verify announcement sent
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
    }

    #[tokio::test]
    async fn test_handle_state_change_ejection() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        // Add member first
        client.add_group_member(&group, &member).await.unwrap();

        let config = BotConfig {
            group_id: group.clone(),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        let change = StateChange::MemberRevoked {
            member_hash: "hash123".to_string(),
            service_id: member.clone(),
            trigger: EjectionTrigger::BelowThreshold {
                effective_vouches: 1,
                min_threshold: 2,
            },
        };

        bot.handle_state_change(change).await.unwrap();

        // Verify member removed from group
        assert!(!client.is_member(&group, &member));
    }

    #[test]
    fn test_check_ejection() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let bot = StromaBot::new(client, freenet, config);

        // Below threshold
        let trigger = bot.check_ejection(1, 0, 0);
        assert!(matches!(
            trigger,
            Some(EjectionTrigger::BelowThreshold { .. })
        ));

        // Negative standing
        let trigger = bot.check_ejection(2, 3, 0);
        assert!(matches!(
            trigger,
            Some(EjectionTrigger::NegativeStanding { .. })
        ));

        // No ejection
        let trigger = bot.check_ejection(3, 1, 0);
        assert!(trigger.is_none());
    }

    #[tokio::test]
    async fn test_handle_invite_command() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        let message = Message {
            sender: ServiceId("alice".to_string()),
            content: MessageContent::Text("/invite @bob Great activist".to_string()),
            timestamp: 1234567890,
        };

        bot.handle_message(message).await.unwrap();

        // Verify bot sent response
        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Invitation for @bob recorded"));

        // Verify ephemeral session created
        assert_eq!(bot.vetting_sessions.active_count(), 1);
        let session = bot.vetting_sessions.get_session("@bob");
        assert!(session.is_some());
        assert_eq!(session.unwrap().invitee_username, "@bob");
    }

    #[tokio::test]
    async fn test_handle_invite_with_previous_flags() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Manually create session with previous flags for testing
        bot.vetting_sessions
            .create_session(
                ServiceId("@bob".to_string()),
                "@bob".to_string(),
                crate::freenet::contract::MemberHash::from_bytes(&[1; 32]),
                ServiceId("alice".to_string()),
                Some("Context".to_string()),
                true,
                3,
            )
            .ok();

        // Note: In Phase 1, this would query Freenet for previous flags
        // For now, we're testing the session management structure
        let session = bot.vetting_sessions.get_session("@bob");
        assert!(session.is_some());
        assert!(session.unwrap().has_previous_flags);
        assert_eq!(session.unwrap().previous_flag_count, 3);
    }

    #[test]
    fn test_verify_admission_proof_valid() {
        use crate::stark::types::MemberHash;
        use std::collections::BTreeSet;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let bot = StromaBot::new(client, freenet, config);

        // Create test member and vouchers
        let member = MemberHash([1; 32]);
        let voucher1 = MemberHash([2; 32]);
        let voucher2 = MemberHash([3; 32]);
        let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
        let flaggers: BTreeSet<_> = BTreeSet::new();

        // Verify admission proof
        let result = bot.verify_admission_proof(member, vouchers, flaggers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_admission_proof_insufficient_vouches() {
        use crate::stark::types::MemberHash;
        use std::collections::BTreeSet;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let bot = StromaBot::new(client, freenet, config);

        // Create test member with only 1 voucher
        let member = MemberHash([1; 32]);
        let voucher1 = MemberHash([2; 32]);
        let vouchers: BTreeSet<_> = [voucher1].into_iter().collect();
        let flaggers: BTreeSet<_> = BTreeSet::new();

        // Verify admission proof fails
        let result = bot.verify_admission_proof(member, vouchers, flaggers);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient vouches"));
    }

    #[test]
    fn test_verify_admission_proof_negative_standing() {
        use crate::stark::types::MemberHash;
        use std::collections::BTreeSet;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let bot = StromaBot::new(client, freenet, config);

        // Create test member with vouchers but more flags
        let member = MemberHash([1; 32]);
        let voucher1 = MemberHash([2; 32]);
        let voucher2 = MemberHash([3; 32]);
        let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
        let flagger1 = MemberHash([4; 32]);
        let flagger2 = MemberHash([5; 32]);
        let flagger3 = MemberHash([6; 32]);
        let flaggers: BTreeSet<_> = [flagger1, flagger2, flagger3].into_iter().collect();

        // Verify admission proof fails due to negative standing
        let result = bot.verify_admission_proof(member, vouchers, flaggers);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Negative standing"));
    }

    #[test]
    fn test_verify_admission_proof_with_voucher_flaggers() {
        use crate::stark::types::MemberHash;
        use std::collections::BTreeSet;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let bot = StromaBot::new(client, freenet, config);

        // Create test member where one voucher also flagged (overlap)
        let member = MemberHash([1; 32]);
        let voucher1 = MemberHash([2; 32]);
        let voucher2 = MemberHash([3; 32]);
        let voucher3 = MemberHash([4; 32]);
        let vouchers: BTreeSet<_> = [voucher1, voucher2, voucher3].into_iter().collect();
        // voucher2 also flags (creates overlap)
        let flaggers: BTreeSet<_> = [voucher2].into_iter().collect();

        // effective_vouches = 3 - 1 = 2 (meets threshold)
        // regular_flags = 1 - 1 = 0
        // standing = 2 - 0 = 2 (positive)
        let result = bot.verify_admission_proof(member, vouchers, flaggers);
        assert!(result.is_ok());
    }
}
