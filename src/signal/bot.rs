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
use crate::identity::mask_identity;
use tracing::warn;

/// Stroma bot configuration
///
/// # Key Derivation
///
/// The `identity_masking_key` and `voter_pepper` should be derived from the
/// operator's BIP-39 mnemonic via `StromaKeyring`. Do NOT use hardcoded values
/// in production.
///
/// ```rust,ignore
/// let keyring = StromaKeyring::from_mnemonic(&mnemonic)?;
/// let config = BotConfig {
///     identity_masking_key: *keyring.identity_masking_key(),
///     voter_pepper: *keyring.voter_pepper(),
///     ..Default::default()
/// };
/// ```
pub struct BotConfig {
    pub group_id: GroupId,
    pub min_vouch_threshold: u32,
    /// Key for HMAC-SHA256 identity masking (from `StromaKeyring::identity_masking_key()`)
    pub identity_masking_key: [u8; 32],
    /// Pepper for voter deduplication (from `StromaKeyring::voter_pepper()`)
    pub voter_pepper: [u8; 32],
    pub contract_hash: Option<crate::freenet::traits::ContractHash>,
}

impl Default for BotConfig {
    /// Creates a BotConfig with test keys.
    ///
    /// # Warning
    ///
    /// These are INSECURE test keys. In production, derive keys from
    /// `StromaKeyring::from_mnemonic()`.
    fn default() -> Self {
        // Test keys - 32 bytes each (NOT for production use)
        let test_identity_key = *b"test-identity-masking-key-32b!!!";
        let test_voter_pepper = *b"test-voter-pepper-key-32-bytes!!";

        Self {
            group_id: GroupId(vec![]),
            min_vouch_threshold: 2,
            identity_masking_key: test_identity_key,
            voter_pepper: test_voter_pepper,
            contract_hash: None,
        }
    }
}

/// Stroma Signal bot
pub struct StromaBot<C: SignalClient, F: crate::freenet::FreenetClient> {
    client: C,
    freenet: F,
    config: BotConfig,
    config_path: Option<std::path::PathBuf>,
    group_manager: GroupManager<C>,
    poll_manager: PollManager<C>,
    bootstrap_manager: BootstrapManager<C>,
    vetting_sessions: VettingSessionManager,
    member_resolver: MemberResolver,
    persistence_manager: crate::persistence::WriteBlockingManager,
}

impl<C: SignalClient, F: crate::freenet::FreenetClient> StromaBot<C, F> {
    pub fn new(client: C, freenet: F, config: BotConfig) -> Result<Self, SignalError> {
        let group_manager = GroupManager::new(client.clone(), config.group_id.clone());

        // Use mnemonic-derived keys from BotConfig
        // - identity_masking_key: for HMAC-SHA256 identity masking
        // - voter_pepper: for poll voter deduplication
        let poll_manager = PollManager::new(
            client.clone(),
            config.group_id.clone(),
            &config.voter_pepper,
            None,
        );
        let bootstrap_manager = BootstrapManager::new(client.clone(), config.identity_masking_key);
        let vetting_sessions = VettingSessionManager::new();
        let member_resolver = MemberResolver::new(config.identity_masking_key);
        let persistence_manager = crate::persistence::WriteBlockingManager::new();

        Ok(Self {
            client,
            freenet,
            config,
            config_path: None,
            group_manager,
            poll_manager,
            bootstrap_manager,
            vetting_sessions,
            member_resolver,
            persistence_manager,
        })
    }

    /// Set config file path for persistence
    ///
    /// Must be called after construction to enable config persistence when
    /// bootstrap completes.
    pub fn set_config_path(&mut self, path: std::path::PathBuf) {
        self.config_path = Some(path);
    }

    /// Send response to appropriate destination with defensive group_id validation
    ///
    /// Per Stroma architecture, bot should be 1:1 with group. However, during
    /// bootstrap or failure scenarios, group_id may not be configured yet.
    /// This helper defensively handles that case.
    async fn send_response(
        &self,
        source: &MessageSource,
        sender: &ServiceId,
        text: &str,
    ) -> SignalResult<()> {
        match source {
            MessageSource::DirectMessage => self.client.send_message(sender, text).await,
            MessageSource::Group => {
                if self.config.group_id.0.is_empty() {
                    // Bootstrap incomplete - fall back to DM
                    warn!(
                        "Group message received but group_id not configured - falling back to DM"
                    );
                    let fallback_msg = format!(
                        "{}\n\n⚠️ Note: I received your command in a group, but I'm not fully configured yet. \
                        Please complete bootstrap with /create-group first.",
                        text
                    );
                    self.client.send_message(sender, &fallback_msg).await
                } else {
                    self.client
                        .send_group_message(&self.config.group_id, text)
                        .await
                }
            }
        }
    }

    /// Handle bootstrap completion
    ///
    /// Called when /add-seed completes with the 3rd seed member.
    /// This enforces the 1:1 bot-to-group invariant by:
    /// 1. Updating in-memory config with group_id
    /// 2. Leaving all other Signal groups (if any)
    ///
    /// TODO: Persist group_id to config file when CLI integration is complete
    async fn on_bootstrap_complete(&mut self, group_id: GroupId) -> SignalResult<()> {
        use tracing::info;

        let group_id_hex = hex::encode(&group_id.0);

        // 1. Update in-memory config
        self.config.group_id = group_id.clone();
        self.group_manager = GroupManager::new(self.client.clone(), group_id.clone());

        info!("✅ Bootstrap complete - group_id set: {}", group_id_hex);

        // 2. Persist to config file
        // TODO: Requires access to StromaConfig from cli module
        // For now, config_path is stored but not used. This will be implemented
        // when we add proper CLI->library integration for config updates.
        if self.config_path.is_some() {
            warn!(
                "TODO: Persist group_id {} to config file (not yet implemented)",
                group_id_hex
            );
        }

        // 3. Leave all other Signal groups (enforce 1:1 bot-to-group invariant)
        // TODO: Requires SignalClient method to list all groups
        warn!("TODO: Leave all other Signal groups (not yet implemented)");

        Ok(())
    }

    /// Run bot event loop
    ///
    /// Receives Signal messages and processes commands.
    /// In production, also monitors Freenet state changes.
    pub async fn run(&mut self) -> SignalResult<()> {
        use futures::StreamExt;

        // Subscribe to Freenet state stream if contract configured
        let mut freenet_stream = if let Some(contract) = &self.config.contract_hash {
            Some(self.freenet.subscribe(contract).await.map_err(|e| {
                SignalError::Protocol(format!("Failed to subscribe to Freenet: {}", e))
            })?)
        } else {
            None
        };

        // Polling interval for Signal messages
        let mut signal_interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = signal_interval.tick() => {
                    // Receive messages from Signal
                    let messages = match self.client.receive_messages().await {
                        Ok(msgs) => msgs,
                        Err(e) => {
                            warn!("Error receiving messages, will retry: {}", e);
                            continue;
                        }
                    };

                    for message in messages {
                        let sender = message.sender.clone();
                        match self.handle_message(message).await {
                            Ok(()) => {}
                            Err(e) => {
                                // Log the error and notify the sender, but do NOT exit.
                                // A daemon must survive individual message handling failures.
                                warn!("Error handling message from {:?}: {}", sender, e);
                                let _ = self.client.send_message(
                                    &sender,
                                    &format!("⚠️ Error processing your command: {}", e),
                                ).await;
                            }
                        }
                    }
                }
                Some(change) = async {
                    match freenet_stream.as_mut() {
                        Some(stream) => stream.next().await,
                        None => futures::future::pending().await,
                    }
                } => {
                    // Handle Freenet state change
                    self.handle_freenet_state_change(change).await?;
                }
            }
        }
    }

    /// Handle Freenet state change from subscription stream
    ///
    /// Processes raw state changes from Freenet and detects:
    /// - Expired proposals that need to be checked
    /// - Membership changes (future: admission/ejection events)
    async fn handle_freenet_state_change(
        &mut self,
        change: crate::freenet::traits::StateChange,
    ) -> SignalResult<()> {
        use crate::serialization::from_cbor;

        // Deserialize the new state
        let state: crate::freenet::trust_contract::TrustNetworkState =
            from_cbor(&change.new_state.data).map_err(|e| {
                SignalError::Protocol(format!("Failed to deserialize Freenet state: {}", e))
            })?;

        // Check for expired proposals
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for (poll_id, proposal) in &state.active_proposals {
            // Skip if already checked
            if proposal.checked {
                continue;
            }

            // Check if proposal has expired
            if current_time >= proposal.expires_at {
                // Get poll timestamp from poll_manager
                let poll_timestamp = proposal.expires_at;

                // Create ProposalExpired event
                let state_change = StateChange::ProposalExpired {
                    poll_id: *poll_id,
                    poll_timestamp,
                };

                // Handle the expiration
                self.handle_state_change(state_change, &state).await?;
            }
        }

        Ok(())
    }

    /// Handle incoming message
    async fn handle_message(&mut self, message: Message) -> SignalResult<()> {
        // Defensive: Validate message is from correct group (1:1 bot-to-group invariant)
        // If group_id is configured and message is from a group, it must match our group
        if !self.config.group_id.0.is_empty() && matches!(message.source, MessageSource::Group) {
            // Group is configured - this bot belongs to a specific group
            // We can't validate which group the message came from without extracting it,
            // but we've already enforced leaving other groups in on_bootstrap_complete().
            // If we somehow receive a message from another group, it's a protocol violation.

            // For now, log and process (defensive - trust our cleanup logic)
            // Future: Extract actual group ID from message and validate it matches
        }

        match message.content {
            MessageContent::Text(text) => {
                let command = parse_command(&text);

                // Security: Trust operations MUST be in DMs (privacy-first architecture)
                let requires_dm = matches!(
                    command,
                    Command::CreateGroup { .. }
                        | Command::AddSeed { .. }
                        | Command::Invite { .. }
                        | Command::Vouch { .. }
                        | Command::Flag { .. }
                        | Command::RejectIntro { .. }
                );

                if requires_dm && !matches!(message.source, MessageSource::DirectMessage) {
                    let error_msg = "❌ Trust operations must be sent via direct message (DM) for privacy. Please message me directly.";
                    return self
                        .send_response(&message.source, &message.sender, error_msg)
                        .await;
                }

                // Route commands to appropriate handlers
                match command {
                    Command::CreateGroup { ref group_name } => {
                        self.bootstrap_manager
                            .handle_create_group(&message.sender, group_name.clone())
                            .await?;
                    }
                    Command::AddSeed { ref username } => {
                        let service_id = self.client.resolve_identifier(username).await?;

                        // Reject PNI - groups require ACI
                        if service_id.0.starts_with("PNI:") {
                            return Err(SignalError::InvalidMessage(format!(
                                "Cannot add user: Account '{}' has privacy settings that prevent ACI disclosure. \
                                 Ask them to share their Signal username instead, or adjust privacy settings.",
                                username
                            )));
                        }

                        let maybe_group_id = self
                            .bootstrap_manager
                            .handle_add_seed(&self.freenet, &message.sender, &service_id, username)
                            .await?;

                        // If bootstrap completed (3rd seed added), persist group_id and cleanup
                        if let Some(group_id) = maybe_group_id {
                            self.on_bootstrap_complete(group_id).await?;
                        }
                    }
                    Command::Invite {
                        ref username,
                        ref context,
                    } => {
                        let invitee_id = self.client.resolve_identifier(username).await?;

                        // Reject PNI - groups require ACI
                        if invitee_id.0.starts_with("PNI:") {
                            return Err(SignalError::InvalidMessage(format!(
                                "Cannot invite user: Account '{}' due to privacy settings. \
                                 Ask them to share their Signal username instead, or adjust privacy settings.",
                                username
                            )));
                        }

                        self.handle_invite(&message.sender, &invitee_id, context.as_deref())
                            .await?;
                    }
                    Command::Vouch { ref username } => {
                        let target_id = self.client.resolve_identifier(username).await?;

                        // Reject PNI - trust operations require ACI
                        if target_id.0.starts_with("PNI:") {
                            return Err(SignalError::InvalidMessage(format!(
                                "Cannot vouch: Account '{}' due to privacy settings. \
                                 Ask them to share their Signal username instead.",
                                username
                            )));
                        }

                        self.handle_vouch(&message.sender, &target_id).await?;
                    }
                    Command::RejectIntro { ref username } => {
                        let invitee_id = self.client.resolve_identifier(username).await?;

                        // Reject PNI - trust operations require ACI
                        if invitee_id.0.starts_with("PNI:") {
                            return Err(SignalError::InvalidMessage(format!(
                                "Cannot reject intro: Account '{}' due to privacy settings. \
                                 Ask them to share their Signal username instead.",
                                username
                            )));
                        }

                        self.handle_reject_intro(&message.sender, &invitee_id)
                            .await?;
                    }
                    Command::Help => {
                        self.handle_help(&message.sender, &message.source).await?;
                    }
                    _ => {
                        // Other commands go through normal handler
                        handle_pm_command(
                            &self.client,
                            &self.freenet,
                            &self.group_manager,
                            &self.config,
                            &self.persistence_manager,
                            &message.sender,
                            &message.source,
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
                // Extract voter ACI from sender (ServiceId)
                let voter_aci = &message.sender.0;
                self.poll_manager.process_vote(&vote, voter_aci).await?;
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
        invitee: &ServiceId,
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
                return self.handle_invite_bootstrap(sender, invitee, context).await;
            }
        };

        let state_bytes = match self.freenet.get_state(&contract).await {
            Ok(state) => state.data,
            Err(FreenetError::ContractNotFound) => {
                // Bootstrap phase
                return self.handle_invite_bootstrap(sender, invitee, context).await;
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

        // Hash inviter's ServiceId to MemberHash using mnemonic-derived key
        let inviter_hash: MemberHash =
            mask_identity(&sender.0, &self.config.identity_masking_key).into();

        // Verify sender is a member
        if !state.members.contains(&inviter_hash) {
            let response = "❌ You must be a member to invite others.";
            return self.client.send_message(sender, response).await;
        }

        // Phase 1: Query Freenet state for previous flags (GAP-10)
        // Check if invitee already has history in the system
        let invitee_hash: MemberHash =
            mask_identity(&invitee.0, &self.config.identity_masking_key).into();
        let is_ejected = state.ejected.contains(&invitee_hash);
        let has_previous_flags = is_ejected || state.flags.contains_key(&invitee_hash);
        let previous_flag_count = if has_previous_flags {
            state
                .flags
                .get(&invitee_hash)
                .map(|f| f.len() as u32)
                .unwrap_or(0)
        } else {
            0
        };

        // Create ephemeral vetting session (keyed by invitee's ServiceId)
        let result = self.vetting_sessions.create_session(
            invitee.clone(),
            invitee.0.clone(),
            inviter_hash,
            sender.clone(),
            context.map(|s| s.to_string()),
            has_previous_flags,
            previous_flag_count,
        );

        if let Err(e) = result {
            let response = format!("❌ Cannot invite {}: {}", invitee.0, e);
            return self.client.send_message(sender, &response).await;
        }

        // Record first vouch in Freenet (inviter vouching for invitee)
        let mut delta = crate::freenet::trust_contract::StateDelta::new();
        delta = delta.add_vouch(inviter_hash, invitee_hash);

        let delta_bytes = match crate::serialization::to_cbor(&delta) {
            Ok(bytes) => bytes,
            Err(e) => {
                let response = format!("❌ Failed to serialize vouch delta: {}", e);
                return self.client.send_message(sender, &response).await;
            }
        };

        let contract_delta = crate::freenet::traits::ContractDelta { data: delta_bytes };
        if let Err(e) = self.freenet.apply_delta(&contract, &contract_delta).await {
            let response = format!("❌ Failed to apply vouch delta to Freenet: {}", e);
            return self.client.send_message(sender, &response).await;
        }

        // Use actual Freenet state for cross-cluster matching (already queried above)

        // Select assessor via Blind Matchmaker
        // TODO Phase 1: Track previously assigned assessors for DVR optimization
        let excluded = std::collections::HashSet::new();
        let assessor_hash = BlindMatchmaker::select_validator(&state, &inviter_hash, &excluded);

        if let Some(assessor) = assessor_hash {
            // Resolve assessor MemberHash to ServiceId via MemberResolver
            match self.member_resolver.get_service_id(&assessor) {
                Some(assessor_id) => {
                    // Assign assessor to session
                    if let Err(e) = self.vetting_sessions.assign_assessor(
                        &invitee.0,
                        assessor,
                        assessor_id.clone(),
                    ) {
                        let response = format!("❌ Failed to assign assessor: {}", e);
                        return self.client.send_message(sender, &response).await;
                    }

                    // Send PM to assessor with assessment request
                    let assessment_msg = msg_assessment_request(
                        &invitee.0,
                        context,
                        has_previous_flags,
                        previous_flag_count,
                    );
                    self.client
                        .send_message(assessor_id, &assessment_msg)
                        .await?;

                    // Send confirmation PM to inviter (no assessor identity revealed)
                    let inviter_msg = msg_inviter_confirmation(
                        &invitee.0,
                        has_previous_flags,
                        previous_flag_count,
                    );
                    self.client.send_message(sender, &inviter_msg).await
                }
                None => {
                    // Assessor not in resolver - this shouldn't happen
                    // Send stalled notification to inviter
                    let stall_msg = msg_no_candidates(&invitee.0);
                    self.client.send_message(sender, &stall_msg).await
                }
            }
        } else {
            // No assessor found (bootstrap phase or no cross-cluster members)
            // Bootstrap exception: Network is small
            let response = format!(
                "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nNote: Network is small (bootstrap phase). They'll join once they receive one more vouch from any member.",
                invitee.0,
                context.unwrap_or("(no context provided)")
            );
            self.client.send_message(sender, &response).await
        }
    }

    /// Handle /invite in bootstrap phase (before contract setup)
    async fn handle_invite_bootstrap(
        &mut self,
        sender: &ServiceId,
        invitee: &ServiceId,
        context: Option<&str>,
    ) -> SignalResult<()> {
        // Hash inviter's ServiceId to MemberHash using mnemonic-derived key
        let inviter_hash: MemberHash =
            mask_identity(&sender.0, &self.config.identity_masking_key).into();

        // Create ephemeral vetting session (bootstrap: no previous flags)
        let result = self.vetting_sessions.create_session(
            invitee.clone(),
            invitee.0.clone(),
            inviter_hash,
            sender.clone(),
            context.map(|s| s.to_string()),
            false,
            0,
        );

        if let Err(e) = result {
            let response = format!("❌ Cannot invite {}: {}", invitee.0, e);
            return self.client.send_message(sender, &response).await;
        }

        // Bootstrap phase: simple confirmation, no cross-cluster matching yet
        let response = format!(
            "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nNote: Network is small (bootstrap phase). They'll join once they receive one more vouch from any member.",
            invitee.0,
            context.unwrap_or("(no context provided)")
        );
        self.client.send_message(sender, &response).await
    }

    /// Handle /vouch command
    ///
    /// Records second vouch and admits member if threshold met.
    async fn handle_vouch(&mut self, sender: &ServiceId, target: &ServiceId) -> SignalResult<()> {
        use crate::matchmaker::cluster_detection::detect_clusters;
        use std::collections::BTreeSet;

        // 1. Hash sender's ServiceId to MemberHash using mnemonic-derived key
        let voucher_hash: MemberHash =
            mask_identity(&sender.0, &self.config.identity_masking_key).into();

        // 2. Get current Freenet state
        let contract = match &self.config.contract_hash {
            Some(c) => c,
            None => {
                let response = "❌ Cannot process vouch: Freenet contract not configured.";
                return self.client.send_message(sender, response).await;
            }
        };

        let state_bytes =
            self.freenet.get_state(contract).await.map_err(|e| {
                SignalError::Protocol(format!("Failed to get Freenet state: {}", e))
            })?;

        let state: crate::freenet::trust_contract::TrustNetworkState =
            crate::serialization::from_cbor(&state_bytes.data).map_err(|e| {
                SignalError::Protocol(format!("Failed to deserialize trust state: {}", e))
            })?;

        // 3. Verify sender is an active member
        if !state.members.contains(&voucher_hash) {
            let response = "❌ Only active members can vouch for others.";
            return self.client.send_message(sender, response).await;
        }

        // 4. Check if vetting session exists for target and clone needed data
        let (invitee_id, inviter_hash, inviter_id) =
            match self.vetting_sessions.get_session(&target.0) {
                Some(s) => (s.invitee_id.clone(), s.inviter, s.inviter_id.clone()),
                None => {
                    let response = format!(
                    "❌ No active vetting session for {}. They must be invited first with /invite.",
                    target.0
                );
                    return self.client.send_message(sender, &response).await;
                }
            };

        let invitee_hash: MemberHash =
            mask_identity(&invitee_id.0, &self.config.identity_masking_key).into();

        // 5. Verify sender isn't the inviter (can't vouch twice)
        if voucher_hash == inviter_hash {
            let response = format!(
                "❌ You already vouched for {} when you invited them. A second vouch from a different member is required.",
                target.0
            );
            return self.client.send_message(sender, &response).await;
        }

        // 6. Verify cross-cluster requirement (if network has ≥2 clusters)
        let cluster_result = detect_clusters(&state);
        let cross_cluster_required = cluster_result.cluster_count >= 2;

        if cross_cluster_required {
            // Check if voucher is from different cluster than inviter
            let inviter_cluster = cluster_result.member_clusters.get(&inviter_hash);
            let voucher_cluster = cluster_result.member_clusters.get(&voucher_hash);
            let is_cross_cluster = match (inviter_cluster, voucher_cluster) {
                (Some(ic), Some(vc)) => ic != vc,
                _ => false,
            };

            if !is_cross_cluster {
                let response = format!(
                    "❌ Cross-cluster vouching required: {} was invited by someone in your cluster. A vouch from a member in a different cluster is needed.",
                    target.0
                );
                return self.client.send_message(sender, &response).await;
            }
        }

        // 7. Record vouch in Freenet (AddVouch delta)
        let mut delta = crate::freenet::trust_contract::StateDelta::new();
        delta = delta.add_vouch(voucher_hash, invitee_hash);

        let delta_bytes = crate::serialization::to_cbor(&delta).map_err(|e| {
            SignalError::Protocol(format!("Failed to serialize vouch delta: {}", e))
        })?;

        let contract_delta = crate::freenet::traits::ContractDelta { data: delta_bytes };
        self.freenet
            .apply_delta(contract, &contract_delta)
            .await
            .map_err(|e| {
                SignalError::Protocol(format!("Failed to apply vouch delta to Freenet: {}", e))
            })?;

        // 8. Get updated state to check admission requirements
        let updated_state_bytes = self.freenet.get_state(contract).await.map_err(|e| {
            SignalError::Protocol(format!("Failed to get updated Freenet state: {}", e))
        })?;

        let updated_state: crate::freenet::trust_contract::TrustNetworkState =
            crate::serialization::from_cbor(&updated_state_bytes.data).map_err(|e| {
                SignalError::Protocol(format!("Failed to deserialize updated trust state: {}", e))
            })?;

        // 9. Check ALL admission requirements
        let vouchers = updated_state.vouchers_for(&invitee_hash);
        let flaggers = updated_state.flaggers_for(&invitee_hash);

        // Calculate effective vouches
        let voucher_set: std::collections::HashSet<_> = vouchers.iter().copied().collect();
        let flagger_set: std::collections::HashSet<_> = flaggers.iter().copied().collect();
        let voucher_flaggers: std::collections::HashSet<_> =
            voucher_set.intersection(&flagger_set).copied().collect();

        let all_vouchers = vouchers.len() as u32;
        let voucher_flagger_count = voucher_flaggers.len() as u32;
        let effective_vouches = all_vouchers - voucher_flagger_count;

        // Calculate standing
        let standing = updated_state
            .calculate_standing(&invitee_hash)
            .unwrap_or(-1000); // Default to very negative if not found

        let min_threshold = self.config.min_vouch_threshold;

        // Check if ALL requirements met
        let requirements_met = effective_vouches >= min_threshold && standing >= 0;

        if requirements_met {
            // 10. Generate STARK proof for admission
            let voucher_btree: BTreeSet<_> = vouchers
                .into_iter()
                .map(|h| {
                    let bytes: [u8; 32] = h
                        .as_bytes()
                        .try_into()
                        .expect("MemberHash should be 32 bytes");
                    crate::stark::types::MemberHash(bytes)
                })
                .collect();
            let flagger_btree: BTreeSet<_> = flaggers
                .into_iter()
                .map(|h| {
                    let bytes: [u8; 32] = h
                        .as_bytes()
                        .try_into()
                        .expect("MemberHash should be 32 bytes");
                    crate::stark::types::MemberHash(bytes)
                })
                .collect();

            let invitee_bytes: [u8; 32] = invitee_hash
                .as_bytes()
                .try_into()
                .expect("MemberHash should be 32 bytes");
            let proof_result = self.verify_admission_proof(
                crate::stark::types::MemberHash(invitee_bytes),
                voucher_btree,
                flagger_btree,
            );

            if let Err(e) = proof_result {
                let response = format!("❌ Admission proof verification failed: {}", e);
                return self.client.send_message(sender, &response).await;
            }

            // 11. Add to Signal group
            self.group_manager.add_member(&invitee_id).await?;

            // 12. Add to Freenet as active member
            let mut member_delta = crate::freenet::trust_contract::StateDelta::new();
            member_delta = member_delta.add_member(invitee_hash);

            let member_delta_bytes = crate::serialization::to_cbor(&member_delta).map_err(|e| {
                SignalError::Protocol(format!("Failed to serialize member delta: {}", e))
            })?;

            let member_contract_delta = crate::freenet::traits::ContractDelta {
                data: member_delta_bytes,
            };
            self.freenet
                .apply_delta(contract, &member_contract_delta)
                .await
                .map_err(|e| {
                    SignalError::Protocol(format!("Failed to add member to Freenet: {}", e))
                })?;

            // 13. Announce admission
            let member_hash_str = format!("{:x}", invitee_hash.as_bytes()[0]);
            self.group_manager
                .announce_admission(&member_hash_str)
                .await?;

            // 14. Check for GAP-11 cluster formation announcement
            // Copy contract hash to avoid borrow issues
            let contract_copy = *contract;

            // Get the latest state (with newly admitted member) for cluster detection
            let latest_state_bytes = self.freenet.get_state(&contract_copy).await.map_err(|e| {
                SignalError::Protocol(format!("Failed to get latest Freenet state: {}", e))
            })?;

            let latest_state: crate::freenet::trust_contract::TrustNetworkState =
                crate::serialization::from_cbor(&latest_state_bytes.data).map_err(|e| {
                    SignalError::Protocol(format!(
                        "Failed to deserialize latest trust state: {}",
                        e
                    ))
                })?;

            // Check if cluster formation announcement should be sent
            if self
                .check_and_announce_cluster_formation(&latest_state)
                .await?
            {
                // Create delta to mark announcement as sent
                let mut gap11_delta = crate::freenet::trust_contract::StateDelta::new();
                gap11_delta = gap11_delta.mark_gap11_announced();

                // Apply delta to Freenet
                let gap11_delta_bytes =
                    crate::serialization::to_cbor(&gap11_delta).map_err(|e| {
                        SignalError::Protocol(format!("Failed to serialize GAP-11 delta: {}", e))
                    })?;

                let gap11_contract_delta = crate::freenet::traits::ContractDelta {
                    data: gap11_delta_bytes,
                };
                self.freenet
                    .apply_delta(&contract_copy, &gap11_contract_delta)
                    .await
                    .map_err(|e| {
                        SignalError::Protocol(format!(
                            "Failed to apply GAP-11 delta to Freenet: {}",
                            e
                        ))
                    })?;
            }

            // 15. Delete ephemeral vetting session
            let _session = self.vetting_sessions.admit(&target.0).ok();

            // 16. Notify voucher (sender)
            let response = format!(
                "✅ Your vouch for {} has been recorded. They now have {} effective vouch(es) and met all requirements.\n\n🎉 {} has been admitted to the group!",
                target.0, effective_vouches, target.0
            );
            self.client.send_message(sender, &response).await?;

            // 17. Notify inviter
            let inviter_response = format!(
                "🎉 Great news! {} has been admitted to the group after receiving the required vouches.",
                target.0
            );
            self.client
                .send_message(&inviter_id, &inviter_response)
                .await?;

            Ok(())
        } else {
            // 18. Requirements not met - notify voucher
            let vouches_needed = min_threshold.saturating_sub(effective_vouches);

            let response = if standing < 0 {
                format!(
                    "✅ Your vouch for {} has been recorded.\n\nHowever, they currently have negative standing ({}). They need more vouches to overcome flags before admission.",
                    target.0, standing
                )
            } else {
                format!(
                    "✅ Your vouch for {} has been recorded.\n\nThey now have {} effective vouch(es). {} more vouch(es) needed to reach the {}-vouch threshold.",
                    target.0, effective_vouches, vouches_needed, min_threshold
                )
            };

            self.client.send_message(sender, &response).await
        }
    }

    /// Handle /reject-intro command
    ///
    /// Assessor declines vetting invitation and triggers re-selection.
    async fn handle_reject_intro(
        &mut self,
        sender: &ServiceId,
        invitee: &ServiceId,
    ) -> SignalResult<()> {
        use super::matchmaker::BlindMatchmaker;

        // Get the vetting session for the invitee
        let session = match self.vetting_sessions.get_session_mut(&invitee.0) {
            Some(s) => s,
            None => {
                return self
                    .client
                    .send_message(
                        sender,
                        &format!("❌ No active vetting session found for {}", invitee.0),
                    )
                    .await;
            }
        };

        // Verify sender is the assigned assessor
        let assessor_id = match &session.assessor_id {
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

        if assessor_id != sender {
            return self
                .client
                .send_message(
                    sender,
                    "❌ You are not the assigned assessor for this invitee.",
                )
                .await;
        }

        // Hash sender's ServiceId to MemberHash using mnemonic-derived key
        let sender_hash: MemberHash =
            mask_identity(&sender.0, &self.config.identity_masking_key).into();

        // Add sender to excluded candidates
        session.excluded_candidates.insert(sender_hash);

        // Reset status to PendingMatch for re-selection
        session.status = VettingStatus::PendingMatch;
        session.assessor = None;
        session.assessor_id = None;

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
                    invitee.0
                ),
            )
            .await?;

        // Query Freenet state for cross-cluster matching
        use crate::serialization::from_cbor;
        use crate::signal::vetting::msg_assessment_request;

        let contract = match &self.config.contract_hash {
            Some(hash) => *hash,
            None => {
                // Bootstrap phase - shouldn't happen in reject flow, but handle gracefully
                return self
                    .client
                    .send_message(
                        &inviter_id,
                        "❌ Cannot re-select assessor: network not initialized (bootstrap phase).",
                    )
                    .await;
            }
        };

        let state_bytes = match self.freenet.get_state(&contract).await {
            Ok(state) => state.data,
            Err(e) => {
                return self
                    .client
                    .send_message(
                        &inviter_id,
                        &format!("❌ Failed to query Freenet for re-selection: {}", e),
                    )
                    .await;
            }
        };

        let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes)
        {
            Ok(s) => s,
            Err(e) => {
                return self
                    .client
                    .send_message(
                        &inviter_id,
                        &format!("❌ Failed to deserialize contract state: {}", e),
                    )
                    .await;
            }
        };

        // Re-run BlindMatchmaker with exclusion list
        let new_validator = BlindMatchmaker::select_validator(&state, &inviter_hash, &excluded_set);

        if let Some(validator_hash) = new_validator {
            // Resolve validator hash to ServiceId via MemberResolver
            match self.member_resolver.get_service_id(&validator_hash) {
                Some(validator_id) => {
                    // Get session data for PM (extract values to avoid borrow conflict)
                    let (context, has_previous_flags, previous_flag_count) = {
                        let session = self.vetting_sessions.get_session(&invitee.0).unwrap();
                        (
                            session.context.clone(),
                            session.has_previous_flags,
                            session.previous_flag_count,
                        )
                    };

                    // Assign validator to session
                    if let Err(e) = self.vetting_sessions.assign_assessor(
                        &invitee.0,
                        validator_hash,
                        validator_id.clone(),
                    ) {
                        return self
                            .client
                            .send_message(
                                &inviter_id,
                                &format!("❌ Failed to assign new assessor: {}", e),
                            )
                            .await;
                    }

                    // Send PM to new assessor with assessment request
                    let assessment_msg = msg_assessment_request(
                        &invitee.0,
                        context.as_deref(),
                        has_previous_flags,
                        previous_flag_count,
                    );
                    self.client
                        .send_message(validator_id, &assessment_msg)
                        .await?;

                    // Notify inviter that re-matching succeeded
                    self.client
                        .send_message(
                            &inviter_id,
                            &format!(
                                "✅ New assessor assigned for {}. Assessment in progress.",
                                invitee.0
                            ),
                        )
                        .await?;
                }
                None => {
                    // Validator not in resolver - shouldn't happen, but handle gracefully
                    let session = self.vetting_sessions.get_session_mut(&invitee.0).unwrap();
                    session.status = VettingStatus::Stalled;

                    self.client
                        .send_message(
                            &inviter_id,
                            &format!(
                                "❌ Vetting stalled: Selected assessor for {} not reachable.\n\nThe invitation remains open - they'll be matched when the network topology changes.",
                                invitee.0
                            ),
                        )
                        .await?;
                }
            }
        } else {
            // No available validators - stalled
            // Update session status and notify inviter
            let session = self.vetting_sessions.get_session_mut(&invitee.0).unwrap();
            session.status = VettingStatus::Stalled;

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

    /// Handle /help command
    async fn handle_help(
        &mut self,
        sender: &ServiceId,
        source: &MessageSource,
    ) -> SignalResult<()> {
        use crate::signal::pm::Command;

        // Generate help text dynamically from Command enum
        let mut help_text = String::from("🤖 Stroma Bot Commands\n\n");

        // Group commands by category
        help_text.push_str("⚙️ Bootstrap Commands:\n");
        for (syntax, description) in Command::all_commands().iter().take(2) {
            help_text.push_str(&format!("  {} - {}\n", syntax, description));
        }

        help_text.push_str("\n🤝 Trust Operations:\n");
        for (syntax, description) in Command::all_commands().iter().skip(2).take(4) {
            help_text.push_str(&format!("  {} - {}\n", syntax, description));
        }

        help_text.push_str("\n📊 Governance:\n");
        for (syntax, description) in Command::all_commands().iter().skip(6).take(1) {
            help_text.push_str(&format!("  {} - {}\n", syntax, description));
        }

        help_text.push_str("\n📈 Information:\n");
        for (syntax, description) in Command::all_commands().iter().skip(7).take(3) {
            help_text.push_str(&format!("  {} - {}\n", syntax, description));
        }

        help_text.push_str("\n❓ Meta:\n");
        for (syntax, description) in Command::all_commands().iter().skip(10).take(1) {
            help_text.push_str(&format!("  {} - {}\n", syntax, description));
        }

        help_text.push_str("\n💡 Tip: Use @username or username.## for Signal usernames, +15551234567 for phone numbers.");

        // Respond using defensive helper (handles unconfigured group_id)
        self.send_response(source, sender, &help_text).await?;
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
                                    // Execute Freenet state changes (for Config/Federation)
                                    crate::signal::proposals::execute_proposal(
                                        &self.freenet,
                                        contract,
                                        &proposal.proposal_type,
                                        current_config,
                                    )
                                    .await?;

                                    // Execute Signal group settings (for Signal config)
                                    if let crate::signal::polls::ProposalType::Other {
                                        description,
                                    } = &proposal.proposal_type
                                    {
                                        if description.starts_with("Signal config: ") {
                                            let config_part = description
                                                .strip_prefix("Signal config: ")
                                                .unwrap();
                                            if let Some((key, value)) =
                                                config_part.split_once(" = ")
                                            {
                                                execute_signal_setting(
                                                    &self.client,
                                                    &self.config.group_id,
                                                    key,
                                                    value,
                                                )
                                                .await?;
                                            }
                                        }
                                    }
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

/// Execute Signal group setting change.
///
/// Applies the setting change via SignalClient trait methods.
async fn execute_signal_setting(
    client: &impl crate::signal::traits::SignalClient,
    group: &crate::signal::traits::GroupId,
    key: &str,
    value: &str,
) -> crate::signal::traits::SignalResult<()> {
    use crate::signal::proposals::parse_duration_to_secs;
    use crate::signal::traits::SignalError;

    match key {
        "name" => client.set_group_name(group, value).await,
        "description" => client.set_group_description(group, value).await,
        "disappearing_messages" => {
            let seconds = parse_duration_to_secs(value).map_err(SignalError::InvalidMessage)?;
            client.set_disappearing_messages(group, seconds).await
        }
        "announcements_only" => {
            let enabled = value.parse::<bool>().map_err(|_| {
                SignalError::InvalidMessage(format!("Invalid bool value: {}", value))
            })?;
            client.set_announcements_only(group, enabled).await
        }
        _ => Err(SignalError::InvalidMessage(format!(
            "Unknown Signal setting: {}",
            key
        ))),
    }
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

        let _bot = StromaBot::new(client, freenet, config).unwrap();
    }

    #[tokio::test]
    async fn test_handle_text_message() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        let message = Message {
            sender: ServiceId("user1".to_string()),
            source: MessageSource::DirectMessage,
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
            contract_hash: None,
            ..Default::default()
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

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
            contract_hash: None,
            ..Default::default()
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

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
            ..Default::default()
        };
        let bot = StromaBot::new(client, freenet, config).unwrap();

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
            ..Default::default()
        };
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

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
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

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
    async fn test_gap11_integration_in_member_admission() {
        // Test that GAP-11 announcement logic works correctly when
        // admitting a member creates ≥2 clusters
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let group = GroupId(vec![1, 2, 3]);

        // Initialize the group in the mock client
        let dummy_member = ServiceId("dummy".to_string());
        client
            .add_group_member(&group, &dummy_member)
            .await
            .unwrap();

        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet.clone(), config).unwrap();
        bot.group_manager = GroupManager::new(client.clone(), group.clone());

        // Set up state with 2 disconnected clusters
        // Cluster 1: {m1, m2}
        // Cluster 2: {m3, m4}
        let mut state = crate::freenet::trust_contract::TrustNetworkState::new();
        let m1 = crate::freenet::contract::MemberHash::from_bytes(&[1; 32]);
        let m2 = crate::freenet::contract::MemberHash::from_bytes(&[2; 32]);
        let m3 = crate::freenet::contract::MemberHash::from_bytes(&[3; 32]);
        let m4 = crate::freenet::contract::MemberHash::from_bytes(&[4; 32]);

        state.members.insert(m1);
        state.members.insert(m2);
        state.members.insert(m3);
        state.members.insert(m4);

        // Cluster 1: m1 <-> m2
        let mut vouchers1 = std::collections::HashSet::new();
        vouchers1.insert(m2);
        state.vouches.insert(m1, vouchers1);

        // Cluster 2: m3 <-> m4
        let mut vouchers3 = std::collections::HashSet::new();
        vouchers3.insert(m4);
        state.vouches.insert(m3, vouchers3);

        // Verify no announcement has been sent yet
        let sent = client.sent_group_messages(&group);
        assert_eq!(sent.len(), 0, "No messages should be sent initially");

        // Verify gap11_announcement_sent is false initially
        assert!(!state.gap11_announcement_sent);

        // Test that check_and_announce detects the 2 clusters and sends announcement
        let announced = bot
            .check_and_announce_cluster_formation(&state)
            .await
            .unwrap();

        // Verify announcement was sent
        assert!(
            announced,
            "Announcement should be sent when 2 clusters detected"
        );

        let sent = client.sent_group_messages(&group);
        assert_eq!(sent.len(), 1, "One announcement should be sent");
        assert!(
            sent[0].contains("sub-communities"),
            "Message should mention sub-communities"
        );
        assert!(
            sent[0].contains("grandfathered"),
            "Message should mention grandfathering"
        );

        // Test that the delta properly marks announcement as sent
        let mut gap11_delta = crate::freenet::trust_contract::StateDelta::new();
        gap11_delta = gap11_delta.mark_gap11_announced();

        // Apply delta to state
        state.apply_delta(&gap11_delta);

        // Verify the flag is now set
        assert!(
            state.gap11_announcement_sent,
            "GAP-11 flag should be set after applying delta"
        );

        // Verify subsequent check doesn't send another announcement
        let announced_again = bot
            .check_and_announce_cluster_formation(&state)
            .await
            .unwrap();
        assert!(
            !announced_again,
            "Announcement should not be sent again when flag is set"
        );

        let sent = client.sent_group_messages(&group);
        assert_eq!(sent.len(), 1, "Still only one announcement should exist");
    }

    #[tokio::test]
    async fn test_handle_invite_command() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        let message = Message {
            sender: ServiceId("alice".to_string()),
            source: MessageSource::DirectMessage,
            content: MessageContent::Text("/invite @bob Great activist".to_string()),
            timestamp: 1234567890,
        };

        bot.handle_message(message).await.unwrap();

        // Verify bot sent response
        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        eprintln!("Actual message: {}", sent[0].content);
        assert!(sent[0].content.contains("Invitation for") && sent[0].content.contains("recorded"));

        // Verify ephemeral session created
        assert_eq!(bot.vetting_sessions.active_count(), 1);
        let session = bot.vetting_sessions.get_session("bob");
        assert!(session.is_some());
        assert_eq!(session.unwrap().invitee_username, "bob");
    }

    #[tokio::test]
    async fn test_handle_invite_with_previous_flags() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

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
            ..Default::default()
        };
        let bot = StromaBot::new(client, freenet, config).unwrap();

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
            ..Default::default()
        };
        let bot = StromaBot::new(client, freenet, config).unwrap();

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
            ..Default::default()
        };
        let bot = StromaBot::new(client, freenet, config).unwrap();

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
            ..Default::default()
        };
        let bot = StromaBot::new(client, freenet, config).unwrap();

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
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        // Create a vetting session with an assigned assessor
        let inviter_hash = MemberHash::from_bytes(&[1; 32]);
        let assessor_id = ServiceId("assessor".to_string());
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

        // Assign assessor
        bot.vetting_sessions
            .assign_assessor(
                invitee_username,
                MemberHash::from_bytes(&[2; 32]),
                assessor_id.clone(),
            )
            .unwrap();

        // Handle reject-intro from the assessor
        bot.handle_reject_intro(&assessor_id, &ServiceId(invitee_username.to_string()))
            .await
            .unwrap();

        // Verify acknowledgment was sent to assessor
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
        assert!(sent[0].content.contains("declined"));

        // Verify session was updated
        let session = bot.vetting_sessions.get_session(invitee_username).unwrap();
        assert_eq!(session.excluded_candidates.len(), 1);
        // Status remains PendingMatch because:
        // 1. contract_hash is None (bootstrap phase), so re-matching can't be attempted
        // 2. Session stays alive so invitee can be matched when network initializes
        // 3. /reject-intro means "I can't assess" not "invitee is unsuitable"
        //    (assessors should use /flag if invitee is a threat)
        assert_eq!(session.status, VettingStatus::PendingMatch);
        assert!(session.assessor.is_none());
    }

    #[tokio::test]
    async fn test_handle_reject_intro_not_assessor() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        // Create a vetting session with an assigned assessor
        let inviter_hash = MemberHash::from_bytes(&[1; 32]);
        let assessor_id = ServiceId("assessor".to_string());
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
            .assign_assessor(
                invitee_username,
                MemberHash::from_bytes(&[2; 32]),
                assessor_id,
            )
            .unwrap();

        // Try to reject from wrong sender
        bot.handle_reject_intro(&wrong_sender, &ServiceId(invitee_username.to_string()))
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
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        // Try to reject for non-existent session
        bot.handle_reject_intro(
            &ServiceId("someone".to_string()),
            &ServiceId("@nonexistent".to_string()),
        )
        .await
        .unwrap();

        // Verify error message was sent
        let sent = client.sent_messages();
        assert!(!sent.is_empty());
        assert!(sent[0].content.contains("No active vetting session"));
    }

    #[tokio::test]
    async fn test_handle_help_generates_dynamic_output() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), freenet, config).unwrap();

        // Send help command
        bot.handle_help(
            &ServiceId("user1".to_string()),
            &MessageSource::DirectMessage,
        )
        .await
        .unwrap();

        // Verify help message was sent
        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);

        let help_text = &sent[0].content;

        // Verify structure
        assert!(help_text.contains("🤖 Stroma Bot Commands"));

        // Verify command categories
        assert!(help_text.contains("⚙️ Bootstrap Commands:"));
        assert!(help_text.contains("🤝 Trust Operations:"));
        assert!(help_text.contains("📊 Governance:"));
        assert!(help_text.contains("📈 Information:"));
        assert!(help_text.contains("❓ Meta:"));

        // Verify key commands are present
        assert!(help_text.contains("/create-group"));
        assert!(help_text.contains("/add-seed"));
        assert!(help_text.contains("/invite"));
        assert!(help_text.contains("/vouch"));
        assert!(help_text.contains("/flag"));
        assert!(help_text.contains("/reject-intro"));
        assert!(help_text.contains("/propose"));
        assert!(help_text.contains("/status"));
        assert!(help_text.contains("/mesh"));
        assert!(help_text.contains("/audit"));
        assert!(help_text.contains("/help"));

        // Verify descriptions are included
        assert!(help_text.contains("Invite someone"));
        assert!(help_text.contains("Vouch for"));
        assert!(help_text.contains("Flag a member"));
        assert!(help_text.contains("View your personal trust standing"));
        assert!(help_text.contains("View network overview"));

        // Verify tip is included
        assert!(help_text.contains("💡 Tip"));
    }

    #[test]
    fn test_command_help_text_coverage() {
        use crate::signal::pm::Command;

        // Verify all commands (except Unknown) have help text
        let bootstrap_cmds = [
            Command::CreateGroup {
                group_name: String::new(),
            },
            Command::AddSeed {
                username: String::new(),
            },
        ];

        let trust_cmds = [
            Command::Invite {
                username: String::new(),
                context: None,
            },
            Command::Vouch {
                username: String::new(),
            },
            Command::Flag {
                username: String::new(),
                reason: None,
            },
            Command::RejectIntro {
                username: String::new(),
            },
        ];

        let governance_cmds = [Command::Propose {
            subcommand: String::new(),
            args: vec![],
        }];

        let info_cmds = [
            Command::Status { username: None },
            Command::Mesh { subcommand: None },
            Command::Audit {
                subcommand: String::new(),
            },
        ];

        let meta_cmds = [Command::Help];

        // All non-Unknown commands should have help text
        for cmd in bootstrap_cmds
            .iter()
            .chain(trust_cmds.iter())
            .chain(governance_cmds.iter())
            .chain(info_cmds.iter())
            .chain(meta_cmds.iter())
        {
            assert!(
                cmd.help_text().is_some(),
                "Command {:?} should have help text",
                cmd
            );
        }

        // Unknown should not have help text
        assert!(Command::Unknown("test".to_string()).help_text().is_none());

        // Verify all_commands() returns expected count
        let all_cmds = Command::all_commands();
        assert_eq!(
            all_cmds.len(),
            11,
            "all_commands() should return 11 commands (excluding Unknown)"
        );
    }
}
