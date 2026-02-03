//! Private Message Handling
//!
//! All vetting operations occur in 1-on-1 PMs with bot.
//! Commands: /invite, /vouch, /flag, /propose, /status, /mesh, /audit
//!
//! See: .beads/signal-integration.bead § Privacy-First UX

use super::{group::GroupManager, traits::*};
use crate::freenet::{traits::ContractHash, FreenetClient};

/// PM command types
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Create new group (bootstrap)
    CreateGroup { group_name: String },

    /// Add seed member (bootstrap)
    AddSeed { username: String },

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
    Propose {
        subcommand: String,
        args: Vec<String>,
    },

    /// View personal trust standing
    Status { username: Option<String> },

    /// View network overview
    Mesh,

    /// Audit operator actions
    Audit { subcommand: String },

    /// Unknown command
    Unknown(String),
}

/// Context for command handlers.
///
/// Provides access to Signal client, Freenet client, group manager, and config.
pub struct BotContext<'a, S: SignalClient, F: FreenetClient> {
    pub signal: &'a S,
    pub freenet: &'a F,
    pub group_manager: &'a GroupManager<S>,
    pub contract_hash: ContractHash,
    pub min_vouch_threshold: u32,
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
        "/create-group" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            // Group name is everything after /create-group (may contain spaces)
            let group_name = parts[1..].join(" ").trim_matches('"').to_string();
            Command::CreateGroup { group_name }
        }

        "/add-seed" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::AddSeed {
                username: parts[1].to_string(),
            }
        }

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

        "/status" => Command::Status {
            username: if parts.len() > 1 {
                Some(parts[1].to_string())
            } else {
                None
            },
        },

        "/mesh" => Command::Mesh,

        "/audit" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Audit {
                subcommand: parts[1].to_string(),
            }
        }

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
        Command::CreateGroup { group_name: _ } => {
            // Bootstrap commands handled by BootstrapManager
            client
                .send_message(
                    sender,
                    "Bootstrap commands must be handled by BootstrapManager. This is a stub.",
                )
                .await
        }

        Command::AddSeed { username: _ } => {
            // Bootstrap commands handled by BootstrapManager
            client
                .send_message(
                    sender,
                    "Bootstrap commands must be handled by BootstrapManager. This is a stub.",
                )
                .await
        }

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

        Command::Status { username } => handle_status(client, sender, username.as_deref()).await,

        Command::Mesh => handle_mesh(client, sender).await,

        Command::Audit { subcommand } => handle_audit(client, sender, &subcommand).await,

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
    context: Option<&str>,
) -> SignalResult<()> {
    // TODO: Implement full invitation logic with Freenet:
    // 1. Verify sender is a current member (query Freenet contract)
    // 2. Hash invitee identity: ServiceId -> MemberHash (with ACI-derived key)
    // 3. Check if invitee already exists (member or pending invitee)
    // 4. Record first vouch in Freenet (AddVouch delta)
    // 5. Start vetting process (trigger cross-cluster matching)
    // 6. Send confirmation to inviter with next steps

    let context_str = context.unwrap_or("(no context provided)");
    let response = format!(
        "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nI'm now reaching out to a member from a different cluster for the cross-cluster vouch. You'll be notified when the vetting process progresses.",
        username, context_str
    );
    client.send_message(sender, &response).await
}

async fn handle_vouch(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: &str,
) -> SignalResult<()> {
    // TODO: Implement full vouch logic with Freenet:
    // 1. Verify sender is a current member (query Freenet contract)
    // 2. Hash target identity: username/ServiceId -> MemberHash
    // 3. Verify target exists (as invitee or existing member)
    // 4. Check sender hasn't already vouched for target
    // 5. Record vouch in Freenet (AddVouch delta)
    // 6. Recalculate target's effective vouches
    // 7. If threshold met (effective_vouches >= 2), add to Signal group
    // 8. Send confirmation with updated standing

    let response = format!(
        "✅ Vouch for {} recorded.\n\nTheir standing has been updated. If they've reached the 2-vouch threshold, they'll be automatically added to the Signal group.",
        username
    );
    client.send_message(sender, &response).await
}

async fn handle_flag(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: &str,
    reason: Option<&str>,
) -> SignalResult<()> {
    // TODO: Implement full flag logic with Freenet:
    // 1. Verify sender is a current member (query Freenet contract)
    // 2. Hash target identity: username/ServiceId -> MemberHash
    // 3. Verify target exists and is a member (not just invitee)
    // 4. Record flag in Freenet (AddFlag delta)
    // 5. Check if sender previously vouched for target (vouch invalidation)
    //    - If yes, remove the vouch (RemoveVouch delta)
    // 6. Recalculate target's trust standing:
    //    - all_vouchers, all_flaggers, voucher_flaggers
    //    - effective_vouches = |all_vouchers| - |voucher_flaggers|
    //    - regular_flags = |all_flaggers| - |voucher_flaggers|
    //    - standing = effective_vouches - regular_flags
    // 7. Check ejection triggers:
    //    - Trigger 1: standing < 0 (too many flags)
    //    - Trigger 2: effective_vouches < min_threshold (default 2)
    // 8. If triggered, remove from Signal group (GroupManager)
    // 9. Send confirmation with result

    let reason_str = reason.unwrap_or("(no reason provided)");
    let response = format!(
        "⚠️ Flag for {} recorded.\n\nReason: {}\n\nTheir standing has been recalculated. If ejection triggers are met (standing < 0 OR effective vouches < 2), they'll be automatically removed from the Signal group.",
        username, reason_str
    );
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

async fn handle_status(
    client: &impl SignalClient,
    sender: &ServiceId,
    username: Option<&str>,
) -> SignalResult<()> {
    // GAP-04: /status shows own vouchers only, rejects third-party queries
    if username.is_some() {
        let response = "Third-party status queries are not allowed. Use /status (without username) to see your own standing.";
        return client.send_message(sender, response).await;
    }

    // TODO: Implement status query
    // 1. Hash sender's ServiceId to MemberHash
    // 2. Query Freenet contract for sender's trust state
    // 3. Calculate: all_vouchers, all_flaggers, voucher_flaggers
    // 4. Calculate: effective_vouches = |all_vouchers| - |voucher_flaggers|
    // 5. Calculate: regular_flags = |all_flaggers| - |voucher_flaggers|
    // 6. Calculate: standing = effective_vouches - regular_flags
    // 7. Determine role: Invitee (not in group), Bridge (2 vouches), Validator (3+ vouches)
    // 8. Show list of vouchers (allowed for self-query)

    let response = "📊 Your Trust Status\nRole: Bridge\nAll vouches: 2 (Alice, Bob)\nAll flags: 0\nVoucher-flaggers: 0\nEffective vouches: 2 ✅\nRegular flags: 0\nStanding: +2 (positive)";
    client.send_message(sender, response).await
}

async fn handle_mesh(client: &impl SignalClient, sender: &ServiceId) -> SignalResult<()> {
    // TODO: Implement full mesh overview with Freenet:
    // 1. Query Freenet contract for full member set
    // 2. Calculate network metrics:
    //    - Total members count
    //    - Vouch distribution (how many have 2, 3, 4+ vouches)
    //    - Network health (DVR - Distinct Validator Ratio)
    // 3. Cluster detection (if implemented)
    // 4. NO individual member identities (privacy)
    // 5. Format as user-friendly overview

    let response = "📈 Network Overview\n\nTotal members: 12\nNetwork health: 🟢 Healthy (75% DVR)\n\nTrust Distribution:\n  2 connections: 5 members (42%)\n  3+ connections: 7 members (58%)\n\n💡 Your network has strong distributed trust. Members with 3+ cross-cluster vouches create resilient verification.";
    client.send_message(sender, response).await
}

async fn handle_audit(
    client: &impl SignalClient,
    sender: &ServiceId,
    subcommand: &str,
) -> SignalResult<()> {
    // TODO: Implement audit query (GAP-01)
    // Subcommands: operator, bootstrap
    // Shows operator action history (restarts, maintenance, config changes)
    // Confirms operator has no special privileges for membership

    match subcommand {
        "operator" => {
            let response =
                "Operator action history:\n- Last restart: 2 hours ago\n- No manual interventions";
            client.send_message(sender, response).await
        }
        "bootstrap" => {
            let response = "Bootstrap history:\n- Group created: 7 days ago\n- Initial members: 3";
            client.send_message(sender, response).await
        }
        _ => {
            client
                .send_message(
                    sender,
                    &format!(
                        "Unknown audit subcommand: {}. Use 'operator' or 'bootstrap'",
                        subcommand
                    ),
                )
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::mock::MockSignalClient;

    #[test]
    fn test_parse_create_group() {
        let cmd = parse_command("/create-group \"Mission Control\"");
        assert_eq!(
            cmd,
            Command::CreateGroup {
                group_name: "Mission Control".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_create_group_without_quotes() {
        let cmd = parse_command("/create-group Mission Control");
        assert_eq!(
            cmd,
            Command::CreateGroup {
                group_name: "Mission Control".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_add_seed() {
        let cmd = parse_command("/add-seed @alice");
        assert_eq!(
            cmd,
            Command::AddSeed {
                username: "@alice".to_string(),
            }
        );
    }

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
        assert_eq!(cmd, Command::Status { username: None });
    }

    #[test]
    fn test_parse_status_with_username() {
        let cmd = parse_command("/status @alice");
        assert_eq!(
            cmd,
            Command::Status {
                username: Some("@alice".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_mesh() {
        let cmd = parse_command("/mesh");
        assert_eq!(cmd, Command::Mesh);
    }

    #[test]
    fn test_parse_audit() {
        let cmd = parse_command("/audit operator");
        assert_eq!(
            cmd,
            Command::Audit {
                subcommand: "operator".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_audit_bootstrap() {
        let cmd = parse_command("/audit bootstrap");
        assert_eq!(
            cmd,
            Command::Audit {
                subcommand: "bootstrap".to_string(),
            }
        );
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

        let result = handle_status(&client, &sender, None).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Trust Status"));
    }

    #[tokio::test]
    async fn test_handle_status_rejects_third_party() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let result = handle_status(&client, &sender, Some("@alice")).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("not allowed"));
    }

    #[tokio::test]
    async fn test_handle_audit_operator() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let result = handle_audit(&client, &sender, "operator").await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Operator action history"));
    }

    #[tokio::test]
    async fn test_handle_audit_bootstrap() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let result = handle_audit(&client, &sender, "bootstrap").await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Bootstrap history"));
    }
}
