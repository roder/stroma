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
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            group_id: GroupId(vec![]),
            min_vouch_threshold: 2,
            pepper: b"default-pepper".to_vec(),
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
                        handle_pm_command(&self.client, &message.sender, command).await?;
                    }
                }
            }

            MessageContent::Poll(_) => {
                // Bot created this poll, ignore
            }

            MessageContent::PollVote(vote) => {
                self.poll_manager.process_vote(&vote).await?;
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
    async fn handle_vouch(
        &mut self,
        sender: &ServiceId,
        username: &str,
    ) -> SignalResult<()> {
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
        assert!(sent.len() > 0);
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
}
