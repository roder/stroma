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
    group::{EjectionTrigger, GroupManager},
    pm::{handle_pm_command, parse_command},
    polls::PollManager,
    traits::*,
};

/// Stroma bot configuration
pub struct BotConfig {
    pub group_id: GroupId,
    pub min_vouch_threshold: u32,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            group_id: GroupId(vec![]),
            min_vouch_threshold: 2,
        }
    }
}

/// Stroma Signal bot
pub struct StromaBot<C: SignalClient> {
    client: C,
    config: BotConfig,
    group_manager: GroupManager<C>,
    poll_manager: PollManager<C>,
}

impl<C: SignalClient> StromaBot<C> {
    pub fn new(client: C, config: BotConfig) -> Self {
        let group_manager = GroupManager::new(client.clone(), config.group_id.clone());
        let poll_manager = PollManager::new(client.clone(), config.group_id.clone());

        Self {
            client,
            config,
            group_manager,
            poll_manager,
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
                handle_pm_command(&self.client, &message.sender, command).await?;
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
    use crate::signal::mock::MockSignalClient;

    #[test]
    fn test_bot_creation() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let config = BotConfig::default();

        let _bot = StromaBot::new(client, config);
    }

    #[tokio::test]
    async fn test_handle_text_message() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let config = BotConfig::default();
        let mut bot = StromaBot::new(client.clone(), config);

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
        let group = GroupId(vec![1, 2, 3]);
        let config = BotConfig {
            group_id: group.clone(),
            min_vouch_threshold: 2,
        };
        let mut bot = StromaBot::new(client.clone(), config);

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
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        // Add member first
        client.add_group_member(&group, &member).await.unwrap();

        let config = BotConfig {
            group_id: group.clone(),
            min_vouch_threshold: 2,
        };
        let mut bot = StromaBot::new(client.clone(), config);

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
        let config = BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            min_vouch_threshold: 2,
        };
        let bot = StromaBot::new(client, config);

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
