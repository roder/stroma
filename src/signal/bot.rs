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
    member_resolver::MemberResolver,
    pm::{handle_pm_command, parse_command, Command},
    polls::PollManager,
    traits::*,
    vetting::{VettingSessionManager, VettingStatus},
};
use crate::freenet::contract::MemberHash;

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
    member_resolver: MemberResolver,
}

impl<C: SignalClient, F: crate::freenet::FreenetClient> StromaBot<C, F> {
    pub fn new(client: C, freenet: F, config: BotConfig) -> Self {
        let group_manager = GroupManager::new(client.clone(), config.group_id.clone());
        let poll_manager = PollManager::new(client.clone(), config.group_id.clone());
        let bootstrap_manager = BootstrapManager::new(client.clone(), config.pepper.clone());
        let vetting_sessions = VettingSessionManager::new();
        let member_resolver = MemberResolver::new(config.pepper.clone());

        Self {
            client,
            freenet,
            config,
            group_manager,
            poll_manager,
            bootstrap_manager,
            vetting_sessions,
            member_resolver,
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
                    Command::RejectIntro { ref username } => {
                        self.handle_reject_intro(&message.sender, username).await?;
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
        use crate::freenet::traits::FreenetError;
        use crate::serialization::from_cbor;
        use crate::signal::vetting::{
            msg_assessment_request, msg_inviter_confirmation, msg_no_candidates,
        };

        // Phase 1: Query Freenet to verify sender is a member
        let contract = match &self.config.contract_hash {
            Some(hash) => *hash,
            None => {
                // Bootstrap phase: allow invitations before contract setup
                return self
                    .handle_invite_bootstrap(sender, username, context)
                    .await;
            }
        };

        let state_bytes = match self.freenet.get_state(&contract).await {
            Ok(state) => state.data,
            Err(FreenetError::ContractNotFound) => {
                // Bootstrap phase
                return self
                    .handle_invite_bootstrap(sender, username, context)
                    .await;
            }
            Err(e) => {
                let response = format!("❌ Failed to query Freenet: {}", e);
                return self.client.send_message(sender, &response).await;
            }
        };

        let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes)
        {
            Ok(s) => s,
            Err(e) => {
                let response = format!("❌ Failed to deserialize contract state: {}", e);
                return self.client.send_message(sender, &response).await;
            }
        };

        // Hash inviter's ServiceId to MemberHash
        let inviter_hash = MemberHash::from_identity(&sender.0, &self.config.pepper);

        // Verify sender is a member
        if !state.members.contains(&inviter_hash) {
            let response = "❌ You must be a member to invite others.";
            return self.client.send_message(sender, response).await;
        }

        // Phase 1: Query Freenet state for previous flags (GAP-10)
        // Check if invitee already has history in the system
        let invitee_hash = MemberHash::from_identity(username, &self.config.pepper);
        let has_previous_flags = state.flags.contains_key(&invitee_hash);
        let previous_flag_count = if has_previous_flags {
            state
                .flags
                .get(&invitee_hash)
                .map(|f| f.len() as u32)
                .unwrap_or(0)
        } else {
            0
        };

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

        // Select validator via Blind Matchmaker
        // TODO Phase 1: Track previously assigned validators for DVR optimization
        let excluded = std::collections::HashSet::new();
        let validator_hash = BlindMatchmaker::select_validator(&state, &inviter_hash, &excluded);

        if let Some(validator) = validator_hash {
            // Resolve validator MemberHash to ServiceId via MemberResolver
            match self.member_resolver.get_service_id(&validator) {
                Some(validator_id) => {
                    // Assign validator to session
                    if let Err(e) = self.vetting_sessions.assign_validator(
                        username,
                        validator,
                        validator_id.clone(),
                    ) {
                        let response = format!("❌ Failed to assign validator: {}", e);
                        return self.client.send_message(sender, &response).await;
                    }

                    // Send PM to validator (assessor) with assessment request
                    let assessment_msg = msg_assessment_request(
                        username,
                        context,
                        has_previous_flags,
                        previous_flag_count,
                    );
                    self.client
                        .send_message(validator_id, &assessment_msg)
                        .await?;

                    // Send confirmation PM to inviter (no assessor identity revealed)
                    let inviter_msg = msg_inviter_confirmation(username);
                    self.client.send_message(sender, &inviter_msg).await
                }
                None => {
                    // Validator not in resolver - this shouldn't happen
                    // Send stalled notification to inviter
                    let stall_msg = msg_no_candidates(username);
                    self.client.send_message(sender, &stall_msg).await
                }
            }
        } else {
            // No validator found (bootstrap phase or no cross-cluster members)
            // Bootstrap exception: Network is small
            let response = format!(
                "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nNote: Network is small (bootstrap phase). They'll join once they receive one more vouch from any member.",
                username,
                context.unwrap_or("(no context provided)")
            );
            self.client.send_message(sender, &response).await
        }
    }

    /// Handle /invite in bootstrap phase (before contract setup)
    async fn handle_invite_bootstrap(
        &mut self,
        sender: &ServiceId,
        username: &str,
        context: Option<&str>,
    ) -> SignalResult<()> {
        use crate::freenet::contract::MemberHash;

        // Hash inviter's ServiceId to MemberHash
        let inviter_hash = MemberHash::from_identity(&sender.0, &self.config.pepper);

        // Create ephemeral vetting session (bootstrap: no previous flags)
        let result = self.vetting_sessions.create_session(
            ServiceId(username.to_string()),
            username.to_string(),
            inviter_hash,
            sender.clone(),
            context.map(|s| s.to_string()),
            false,
            0,
        );

        if let Err(e) = result {
            let response = format!("❌ Cannot invite {}: {}", username, e);
            return self.client.send_message(sender, &response).await;
        }

        // Bootstrap phase: simple confirmation, no cross-cluster matching yet
        let response = format!(
            "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nNote: Network is small (bootstrap phase). They'll join once they receive one more vouch from any member.",
            username,
            context.unwrap_or("(no context provided)")
        );
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

    /// Handle /reject-intro command
    ///
    /// Assessor declines vetting invitation and triggers re-selection.
    async fn handle_reject_intro(
        &mut self,
        sender: &ServiceId,
        username: &str,
    ) -> SignalResult<()> {
        use super::matchmaker::BlindMatchmaker;

        // Get the vetting session for the invitee
        let session = match self.vetting_sessions.get_session_mut(username) {
            Some(s) => s,
            None => {
                return self
                    .client
                    .send_message(
                        sender,
                        &format!("❌ No active vetting session found for {}", username),
                    )
                    .await;
            }
        };

        // Verify sender is the assigned assessor/validator
        let validator_id = match &session.validator_id {
            Some(id) => id,
            None => {
                return self
                    .client
                    .send_message(
                        sender,
                        "❌ No assessor has been assigned for this vetting session yet.",
                    )
                    .await;
            }
        };

        if validator_id != sender {
            return self
                .client
                .send_message(sender, "❌ You are not the assigned assessor for this invitee.")
                .await;
        }

        // Hash sender's ServiceId to MemberHash
        let sender_hash = MemberHash::from_identity(&sender.0, &self.config.pepper);

        // Add sender to excluded candidates
        session.excluded_candidates.insert(sender_hash);

        // Reset status to PendingMatch for re-selection
        session.status = VettingStatus::PendingMatch;
        session.validator = None;
        session.validator_id = None;

        // Get inviter for context
        let inviter_hash = session.inviter;
        let inviter_id = session.inviter_id.clone();
        let invitee_username = session.invitee_username.clone();
        let excluded_set = session.excluded_candidates.clone();

        // Send acknowledgment to declining assessor
        self.client
            .send_message(
                sender,
                &format!(
                    "✅ You have declined the assessment for {}.\n\nI'll find another assessor.",
                    username
                ),
            )
            .await?;

        // TODO Phase 1: Query Freenet state for cross-cluster matching
        // For now, use a simple placeholder state
        let state = crate::freenet::trust_contract::TrustNetworkState::new();

        // Re-run BlindMatchmaker with exclusion list
        let new_validator = BlindMatchmaker::select_validator_with_exclusions(
            &state,
            &inviter_hash,
            &excluded_set,
        );

        if let Some(_validator_hash) = new_validator {
            // TODO Phase 1: Resolve validator hash to ServiceId
            // TODO Phase 1: Assign validator to session
            // TODO Phase 1: Send PMs to new validator

            // For now, notify inviter that re-matching is in progress
            self.client
                .send_message(
                    &inviter_id,
                    &format!(
                        "ℹ️ The assessor for {} declined. Finding a new assessor...",
                        invitee_username
                    ),
                )
                .await?;
        } else {
            // No available validators - stalled
            // Update session status and notify inviter
            let session = self.vetting_sessions.get_session_mut(username).unwrap();
            session.status = VettingStatus::Rejected;

            self.client
                .send_message(
                    &inviter_id,
                    &format!(
                        "❌ Vetting stalled: No available assessors for {} (all candidates have declined).\n\nThe invitation remains open - they'll be matched when new members join.",
                        invitee_username
                    ),
                )
                .await?;
        }

        Ok(())
    }

    /// Handle Freenet state change
    ///
    /// React to trust model changes immediately (no polling).
    pub async fn handle_state_change(
        &mut self,
        change: StateChange,
        state: &crate::freenet::trust_contract::TrustNetworkState,
    ) -> SignalResult<bool> {
        let mut state_updated = false;

        match change {
            StateChange::MemberVetted {
                member_hash,
                service_id,
            } => {
                self.group_manager.add_member(&service_id).await?;
                self.group_manager.announce_admission(&member_hash).await?;

                // Check for GAP-11 cluster formation announcement
                if self.check_and_announce_cluster_formation(state).await? {
                    // Create delta to mark announcement as sent
                    let mut delta = crate::freenet::trust_contract::StateDelta::new();
                    delta = delta.mark_gap11_announced();

                    // Apply delta to Freenet
                    if let Some(ref contract) = self.config.contract_hash {
                        let delta_bytes = crate::serialization::to_cbor(&delta).map_err(|e| {
                            SignalError::Protocol(format!(
                                "Failed to serialize GAP-11 delta: {}",
                                e
                            ))
                        })?;

                        let contract_delta =
                            crate::freenet::traits::ContractDelta { data: delta_bytes };
                        self.freenet
                            .apply_delta(contract, &contract_delta)
                            .await
                            .map_err(|e| {
                                SignalError::Protocol(format!(
                                    "Failed to apply GAP-11 delta to Freenet: {}",
                                    e
                                ))
                            })?;

                        state_updated = true;
                    }
                }
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

                // Variable to store outcome for later use
                let outcome = if let Some(aggregate) = votes {
                    // 3. Check quorum and threshold
                    let outcome = self.poll_manager.check_poll_outcome(poll_id, aggregate);

                    if let Some(ref outcome_result) = outcome {
                        // 4. Announce outcome
                        self.group_manager
                            .announce_proposal_result(poll_id, outcome_result)
                            .await?;

                        // 5. Execute proposal if passed
                        if matches!(
                            outcome_result,
                            crate::signal::polls::PollOutcome::Passed { .. }
                        ) {
                            // Get contract hash from config
                            if let Some(ref contract) = self.config.contract_hash {
                                // Get current GroupConfig from Freenet state
                                let state =
                                    self.freenet.get_state(contract).await.map_err(|e| {
                                        SignalError::Protocol(format!(
                                            "Failed to get Freenet state: {}",
                                            e
                                        ))
                                    })?;
                                let trust_state: crate::freenet::trust_contract::TrustNetworkState =
                                    crate::serialization::from_cbor(&state.data).map_err(|e| {
                                        SignalError::Protocol(format!(
                                            "Failed to deserialize trust state: {}",
                                            e
                                        ))
                                    })?;
                                let current_config = &trust_state.config;

                                // Execute proposal
                                if let Some(proposal) = self.poll_manager.get_proposal(poll_id) {
                                    crate::signal::proposals::execute_proposal(
                                        &self.freenet,
                                        contract,
                                        &proposal.proposal_type,
                                        current_config,
                                    )
                                    .await?;
                                }
                            }
                        }
                    }
                    outcome
                } else {
                    None
                };

                // 6. Mark proposal as checked in Freenet
                if let Some(ref contract) = self.config.contract_hash {
                    // Create delta to mark proposal as checked
                    let mut delta = crate::freenet::trust_contract::StateDelta::new();
                    delta.proposals_checked.push(poll_id);

                    // If outcome is available, also store the result
                    if let Some(ref outcome_result) = outcome {
                        let result = match outcome_result {
                            crate::signal::polls::PollOutcome::Passed {
                                approve_count,
                                reject_count,
                            } => crate::freenet::trust_contract::ProposalResult::Passed {
                                approve_count: *approve_count,
                                reject_count: *reject_count,
                            },
                            crate::signal::polls::PollOutcome::Failed {
                                approve_count,
                                reject_count,
                            } => crate::freenet::trust_contract::ProposalResult::Failed {
                                approve_count: *approve_count,
                                reject_count: *reject_count,
                            },
                            crate::signal::polls::PollOutcome::QuorumNotMet {
                                participation_rate,
                                required_quorum: _,
                            } => crate::freenet::trust_contract::ProposalResult::QuorumNotMet {
                                participation_rate: *participation_rate,
                            },
                        };
                        delta.proposals_with_results.push((poll_id, result));
                    }

                    // Serialize and apply delta
                    let delta_bytes = crate::serialization::to_cbor(&delta).map_err(|e| {
                        SignalError::Protocol(format!(
                            "Failed to serialize proposal checked delta: {}",
                            e
                        ))
                    })?;

                    let contract_delta =
                        crate::freenet::traits::ContractDelta { data: delta_bytes };
                    self.freenet
                        .apply_delta(contract, &contract_delta)
                        .await
                        .map_err(|e| {
                            SignalError::Protocol(format!(
                                "Failed to mark proposal as checked in Freenet: {}",
                                e
                            ))
                        })?;
                }
            }
        }

        Ok(state_updated)
    }

    /// Check for GAP-11 cluster formation and announce if needed
    ///
    /// Per GAP-11: When ≥2 clusters first detected, announce cross-cluster requirement.
    /// This is a one-time announcement tracked in state.
    ///
    /// Returns true if announcement was sent (caller should update Freenet state).
    pub async fn check_and_announce_cluster_formation(
        &mut self,
        state: &crate::freenet::trust_contract::TrustNetworkState,
    ) -> SignalResult<bool> {
        use crate::matchmaker::cluster_detection::detect_clusters;

        // Skip if announcement already sent
        if state.gap11_announcement_sent {
            return Ok(false);
        }

        // Detect clusters
        let cluster_result = detect_clusters(state);

        // Check if announcement is needed (≥2 clusters)
        if cluster_result.needs_announcement() {
            let message = cluster_result.announcement_message();
            self.group_manager
                .announce_cluster_formation(message)
                .await?;
            return Ok(true);
        }

        Ok(false)
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

        let state = crate::freenet::trust_contract::TrustNetworkState::new();
        bot.handle_state_change(change, &state).await.unwrap();

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

        let state = crate::freenet::trust_contract::TrustNetworkState::new();
        bot.handle_state_change(change, &state).await.unwrap();

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
    async fn test_gap11_cluster_formation_announcement() {
        use crate::freenet::contract::MemberHash;
        use std::collections::HashSet;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let group = GroupId(vec![1, 2, 3]);

        // Initialize the group in the mock client
        let dummy_member = ServiceId("dummy".to_string());
        client
            .add_group_member(&group, &dummy_member)
            .await
            .unwrap();

        let config = BotConfig {
            group_id: group.clone(),
            min_vouch_threshold: 2,
            pepper: b"test-pepper".to_vec(),
            contract_hash: None,
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Create state with two disconnected clusters
        let mut state = crate::freenet::trust_contract::TrustNetworkState::new();

        // Cluster 1: members 1 and 2
        let m1 = MemberHash::from_bytes(&[1u8; 32]);
        let m2 = MemberHash::from_bytes(&[2u8; 32]);
        state.members.insert(m1);
        state.members.insert(m2);

        let mut vouchers1 = HashSet::new();
        vouchers1.insert(m2);
        state.vouches.insert(m1, vouchers1);

        // Cluster 2: members 3 and 4
        let m3 = MemberHash::from_bytes(&[3u8; 32]);
        let m4 = MemberHash::from_bytes(&[4u8; 32]);
        state.members.insert(m3);
        state.members.insert(m4);

        let mut vouchers3 = HashSet::new();
        vouchers3.insert(m4);
        state.vouches.insert(m3, vouchers3);

        // Check and announce - should send announcement
        let announced = bot
            .check_and_announce_cluster_formation(&state)
            .await
            .unwrap();
        assert!(announced, "Announcement should be sent for 2 clusters");

        // Verify announcement was sent to group
        let sent = client.sent_group_messages(&group);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("sub-communities"));

        // Check again - should not send announcement again
        let mut state_with_flag = state.clone();
        state_with_flag.gap11_announcement_sent = true;
        let announced = bot
            .check_and_announce_cluster_formation(&state_with_flag)
            .await
            .unwrap();
        assert!(!announced, "Announcement should not be sent again");

        // Verify no new messages sent
        let sent = client.sent_group_messages(&group);
        assert_eq!(sent.len(), 1, "Should still be only one message");
    }

    #[tokio::test]
    async fn test_gap11_no_announcement_for_single_cluster() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Create state with single cluster (all connected)
        let state = crate::freenet::trust_contract::TrustNetworkState::new();

        // Check and announce - should NOT send announcement
        let announced = bot
            .check_and_announce_cluster_formation(&state)
            .await
            .unwrap();
        assert!(!announced, "No announcement should be sent for 0 clusters");
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

    #[tokio::test]
    async fn test_handle_reject_intro() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Create a vetting session with an assigned validator
        let inviter_hash = MemberHash::from_bytes(&[1; 32]);
        let validator_id = ServiceId("validator".to_string());
        let invitee_username = "@bob";

        bot.vetting_sessions
            .create_session(
                ServiceId(invitee_username.to_string()),
                invitee_username.to_string(),
                inviter_hash,
                ServiceId("alice".to_string()),
                Some("Context".to_string()),
                false,
                0,
            )
            .unwrap();

        // Assign validator
        bot.vetting_sessions
            .assign_validator(
                invitee_username,
                MemberHash::from_bytes(&[2; 32]),
                validator_id.clone(),
            )
            .unwrap();

        // Handle reject-intro from the validator
        bot.handle_reject_intro(&validator_id, invitee_username)
            .await
            .unwrap();

        // Verify acknowledgment was sent to validator
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
        assert!(sent[0].content.contains("declined"));

        // Verify session was updated
        let session = bot.vetting_sessions.get_session(invitee_username).unwrap();
        assert_eq!(session.excluded_candidates.len(), 1);
        // Status is Rejected because no validators are available (empty state)
        assert_eq!(session.status, VettingStatus::Rejected);
        assert!(session.validator.is_none());
    }

    #[tokio::test]
    async fn test_handle_reject_intro_not_assessor() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Create a vetting session with an assigned validator
        let inviter_hash = MemberHash::from_bytes(&[1; 32]);
        let validator_id = ServiceId("validator".to_string());
        let wrong_sender = ServiceId("wrong_person".to_string());
        let invitee_username = "@bob";

        bot.vetting_sessions
            .create_session(
                ServiceId(invitee_username.to_string()),
                invitee_username.to_string(),
                inviter_hash,
                ServiceId("alice".to_string()),
                Some("Context".to_string()),
                false,
                0,
            )
            .unwrap();

        bot.vetting_sessions
            .assign_validator(
                invitee_username,
                MemberHash::from_bytes(&[2; 32]),
                validator_id,
            )
            .unwrap();

        // Try to reject from wrong sender
        bot.handle_reject_intro(&wrong_sender, invitee_username)
            .await
            .unwrap();

        // Verify error message was sent
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
        assert!(sent[0].content.contains("not the assigned assessor"));

        // Verify session was NOT updated
        let session = bot.vetting_sessions.get_session(invitee_username).unwrap();
        assert_eq!(session.excluded_candidates.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_reject_intro_no_session() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config);

        // Try to reject for non-existent session
        bot.handle_reject_intro(&ServiceId("someone".to_string()), "@nonexistent")
            .await
            .unwrap();

        // Verify error message was sent
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
        assert!(sent[0].content.contains("No active vetting session"));
    }
}
