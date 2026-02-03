//! Private Message Handling
//!
//! All vetting operations occur in 1-on-1 PMs with bot.
//! Commands: /invite, /vouch, /flag, /propose, /status, /mesh
//!
//! See: .beads/signal-integration.bead § Privacy-First UX

use super::traits::*;

/// PM command types
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Invite someone (counts as first vouch)
    Invite {
        username: String,
        context: Option<String>,
    },

    /// Vouch for invitee or existing member
    Vouch { username: String },

    /// Flag member for trust violation
    Flag {
        username: String,
        reason: Option<String>,
    },

    /// Propose group decision
    Propose { subcommand: String, args: Vec<String> },

    /// View personal trust standing
    Status,

    /// View network overview
    Mesh,

    /// Unknown command
    Unknown(String),
}

/// Parse command from message text
pub fn parse_command(text: &str) -> Command {
    let text = text.trim();

    if !text.starts_with('/') {
        return Command::Unknown(text.to_string());
    }

    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return Command::Unknown(text.to_string());
    }

    match parts[0] {
        "/invite" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Invite {
                username: parts[1].to_string(),
                context: if parts.len() > 2 {
                    Some(parts[2..].join(" "))
                } else {
                    None
                },
            }
        }

        "/vouch" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Vouch {
                username: parts[1].to_string(),
            }
        }

        "/flag" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Flag {
                username: parts[1].to_string(),
                reason: if parts.len() > 2 {
                    Some(parts[2..].join(" "))
                } else {
                    None
                },
            }
        }

        "/propose" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Propose {
                subcommand: parts[1].to_string(),
                args: parts[2..].iter().map(|s| s.to_string()).collect(),
            }
        }

        "/status" => Command::Status,

        "/mesh" => Command::Mesh,

        _ => Command::Unknown(text.to_string()),
    }
}

/// Handle PM command
pub async fn handle_pm_command(
    client: &impl SignalClient,
    sender: &ServiceId,
    command: Command,
) -> SignalResult<()> {
    match command {
        Command::Invite { username, context } => {
            handle_invite(client, sender, &username, context.as_deref()).await
        }

        Command::Vouch { username } => handle_vouch(client, sender, &username).await,

        Command::Flag { username, reason } => {
            handle_flag(client, sender, &username, reason.as_deref()).await
        }

        Command::Propose { subcommand, args } => {
            handle_propose(client, sender, &subcommand, &args).await
        }

        Command::Status => handle_status(client, sender).await,

        Command::Mesh => handle_mesh(client, sender).await,

        Command::Unknown(text) => {
            client
                .send_message(sender, &format!("Unknown command: {}", text))
                .await
        }
    }
}

async fn handle_invite(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: &str,
    _context: Option<&str>,
) -> SignalResult<()> {
    // TODO: Implement invitation logic
    // 1. Verify sender is in group (can vouch)
    // 2. Hash invitee identity (with ACI-derived key)
    // 3. Record first vouch in Freenet
    // 4. Start vetting process
    // 5. Send confirmation to inviter

    let response = format!("Invitation for {} recorded (first vouch).", username);
    client.send_message(sender, &response).await
}

async fn handle_vouch(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: &str,
) -> SignalResult<()> {
    // TODO: Implement vouch logic
    // 1. Verify sender is in group
    // 2. Verify target exists (invitee or member)
    // 3. Record vouch in Freenet
    // 4. Check if 2-vouch threshold met
    // 5. Add to Signal group if threshold met

    let response = format!("Vouch for {} recorded.", username);
    client.send_message(sender, &response).await
}

async fn handle_flag(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: &str,
    _reason: Option<&str>,
) -> SignalResult<()> {
    // TODO: Implement flag logic
    // 1. Verify sender is in group
    // 2. Record flag in Freenet
    // 3. Check ejection triggers (standing < 0 or effective_vouches < 2)
    // 4. Remove from Signal group if trigger met

    let response = format!("Flag for {} recorded.", username);
    client.send_message(sender, &response).await
}

async fn handle_propose(
    client: &impl SignalClient,
    sender: &ServiceId,
    subcommand: &str,
    _args: &[String],
) -> SignalResult<()> {
    // TODO: Implement proposal logic
    // Different types: config-change, federation, etc.

    let response = format!("Proposal {} created.", subcommand);
    client.send_message(sender, &response).await
}

async fn handle_status(client: &impl SignalClient, sender: &ServiceId) -> SignalResult<()> {
    // TODO: Implement status query
    // 1. Get member trust standing from Freenet
    // 2. Show: effective_vouches, regular_flags, standing
    // 3. Show: who vouched for them (self-query allowed)

    let response = "Trust standing:\n- Effective vouches: 2\n- Regular flags: 0\n- Standing: 2";
    client.send_message(sender, response).await
}

async fn handle_mesh(client: &impl SignalClient, sender: &ServiceId) -> SignalResult<()> {
    // TODO: Implement mesh overview
    // 1. Get network topology from Freenet
    // 2. Show: total members, cluster distribution
    // 3. NO individual member identities

    let response = "Network overview:\n- Total members: 10\n- Clusters: 3";
    client.send_message(sender, response).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::mock::MockSignalClient;

    #[test]
    fn test_parse_invite() {
        let cmd = parse_command("/invite @alice Great activist");
        assert_eq!(
            cmd,
            Command::Invite {
                username: "@alice".to_string(),
                context: Some("Great activist".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_vouch() {
        let cmd = parse_command("/vouch @alice");
        assert_eq!(
            cmd,
            Command::Vouch {
                username: "@alice".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_flag() {
        let cmd = parse_command("/flag @bob Spam");
        assert_eq!(
            cmd,
            Command::Flag {
                username: "@bob".to_string(),
                reason: Some("Spam".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_status() {
        let cmd = parse_command("/status");
        assert_eq!(cmd, Command::Status);
    }

    #[test]
    fn test_parse_mesh() {
        let cmd = parse_command("/mesh");
        assert_eq!(cmd, Command::Mesh);
    }

    #[test]
    fn test_parse_unknown() {
        let cmd = parse_command("/unknown");
        assert!(matches!(cmd, Command::Unknown(_)));
    }

    #[tokio::test]
    async fn test_handle_invite() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let result = handle_invite(&client, &sender, "@alice", Some("context")).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_status() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let result = handle_status(&client, &sender).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
    }
}
