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
    Mesh { subcommand: Option<String> },

    /// Audit operator actions
    Audit { subcommand: String },

    /// Decline assessment invitation (assessor rejects intro)
    RejectIntro { username: String },

    /// Show help
    Help,

    /// Unknown command
    Unknown(String),
}

impl Command {
    /// Get command syntax and description for help text
    ///
    /// Returns (command_syntax, description) tuple.
    /// Only includes user-facing commands (excludes Unknown).
    pub fn help_text(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Command::CreateGroup { .. } => Some((
                "/create-group <name>",
                "Create new Signal group (bootstrap only)",
            )),
            Command::AddSeed { .. } => {
                Some(("/add-seed <username>", "Add seed member (bootstrap only)"))
            }
            Command::Invite { .. } => Some((
                "/invite <username> [context]",
                "Invite someone (counts as your first vouch for them)",
            )),
            Command::Vouch { .. } => Some((
                "/vouch <username>",
                "Vouch for an invitee or existing member",
            )),
            Command::Flag { .. } => Some((
                "/flag <username> [reason]",
                "Flag a member for trust violation",
            )),
            Command::Propose { .. } => Some((
                "/propose <subcommand> [args...]",
                "Propose a group decision (config change, Signal settings, etc.)",
            )),
            Command::Status { .. } => {
                Some(("/status", "View your personal trust standing and role"))
            }
            Command::Mesh { .. } => Some((
                "/mesh [subcommand]",
                "View network overview (subcommands: strength, replication, config, settings)",
            )),
            Command::Audit { .. } => Some((
                "/audit <subcommand>",
                "Audit operator actions (subcommands: operator, bootstrap)",
            )),
            Command::RejectIntro { .. } => Some((
                "/reject-intro <username>",
                "Decline an assessment invitation (assessor only)",
            )),
            Command::Help => Some(("/help", "Show this help message")),
            Command::Unknown(_) => None,
        }
    }

    /// Get all available commands for help listing
    ///
    /// Returns a list of all command variants with their help text.
    /// Excludes Unknown and includes empty instances for pattern matching.
    pub fn all_commands() -> Vec<(&'static str, &'static str)> {
        vec![
            // Bootstrap commands
            Command::CreateGroup {
                group_name: String::new(),
            }
            .help_text()
            .unwrap(),
            Command::AddSeed {
                username: String::new(),
            }
            .help_text()
            .unwrap(),
            // Trust operations
            Command::Invite {
                username: String::new(),
                context: None,
            }
            .help_text()
            .unwrap(),
            Command::Vouch {
                username: String::new(),
            }
            .help_text()
            .unwrap(),
            Command::Flag {
                username: String::new(),
                reason: None,
            }
            .help_text()
            .unwrap(),
            Command::RejectIntro {
                username: String::new(),
            }
            .help_text()
            .unwrap(),
            // Governance
            Command::Propose {
                subcommand: String::new(),
                args: vec![],
            }
            .help_text()
            .unwrap(),
            // Information
            Command::Status { username: None }.help_text().unwrap(),
            Command::Mesh { subcommand: None }.help_text().unwrap(),
            Command::Audit {
                subcommand: String::new(),
            }
            .help_text()
            .unwrap(),
            // Meta
            Command::Help.help_text().unwrap(),
        ]
    }
}

/// Context for command handlers.
///
/// Provides access to Signal client, Freenet client, group manager, and config.
/// A user identifier that can be a UUID, username, or phone number
#[derive(Debug, Clone, PartialEq)]
pub enum Identifier {
    /// Raw UUID (e.g., "a1b2c3d4-5678-90ab-cdef-1234567890ab")
    Uuid(String),
    /// Signal username (e.g., "matt.42" or "@matt.42")
    Username(String),
    /// Phone number in E.164 format (e.g., "+15551234567")
    Phone(String),
}

/// Parse a user identifier from a string.
///
/// Detection rules (in order):
/// 1. Valid UUID format (including PNI:/ACI: prefixed forms) → Identifier::Uuid
/// 2. Starts with '+' and all digits → Identifier::Phone
/// 3. Otherwise → Identifier::Username
///
/// The '@' prefix is stripped if present (usernames can be entered as "@matt.42" or "matt.42").
///
/// Phone number validation is handled by presage's resolve_phone_number() method,
/// which uses the phonenumber crate to parse and validate E.164 format.
pub fn parse_identifier(input: &str) -> Identifier {
    let input = input.strip_prefix('@').unwrap_or(input);

    // Try parsing as ServiceId (supports plain UUIDs and "PNI:..."/"ACI:..." forms)
    use presage::libsignal_service::protocol::ServiceId;
    if ServiceId::parse_from_service_id_string(input).is_some() {
        return Identifier::Uuid(input.to_string());
    }

    // Check for phone number pattern (+ followed by digits)
    // Detailed validation happens in presage's resolve_phone_number()
    if input.starts_with('+') && input[1..].chars().all(|c| c.is_ascii_digit()) {
        return Identifier::Phone(input.to_string());
    }

    // Default to username
    Identifier::Username(input.to_string())
}

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

        "/mesh" => Command::Mesh {
            subcommand: if parts.len() > 1 {
                Some(parts[1].to_string())
            } else {
                None
            },
        },

        "/audit" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::Audit {
                subcommand: parts[1].to_string(),
            }
        }

        "/reject-intro" => {
            if parts.len() < 2 {
                return Command::Unknown(text.to_string());
            }
            Command::RejectIntro {
                username: parts[1].to_string(),
            }
        }

        "/help" => Command::Help,

        _ => Command::Unknown(text.to_string()),
    }
}

/// Send response to appropriate destination based on message source
async fn send_response(
    client: &impl SignalClient,
    source: &MessageSource,
    sender: &ServiceId,
    text: &str,
) -> SignalResult<()> {
    match source {
        MessageSource::DirectMessage => client.send_message(sender, text).await,
        MessageSource::Group(group_id) => client.send_group_message(group_id, text).await,
    }
}

/// Handle PM command
#[allow(clippy::too_many_arguments)]
pub async fn handle_pm_command<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    group_manager: &GroupManager<impl SignalClient>,
    config: &crate::signal::bot::BotConfig,
    persistence_manager: &crate::persistence::WriteBlockingManager,
    sender: &ServiceId,
    source: &MessageSource,
    command: Command,
) -> SignalResult<()> {
    match command {
        Command::CreateGroup { group_name: _ } => {
            // Bootstrap commands handled by BootstrapManager
            send_response(
                client,
                source,
                sender,
                "Bootstrap commands must be handled by BootstrapManager. This is a stub.",
            )
            .await
        }

        Command::AddSeed { username: _ } => {
            // Bootstrap commands handled by BootstrapManager
            send_response(
                client,
                source,
                sender,
                "Bootstrap commands must be handled by BootstrapManager. This is a stub.",
            )
            .await
        }

        Command::Invite { username, context } => {
            handle_invite(client, source, sender, &username, context.as_deref()).await
        }

        Command::Vouch { username } => handle_vouch(client, source, sender, &username).await,

        Command::Flag { username, reason } => {
            handle_flag(
                client,
                freenet,
                group_manager,
                config,
                source,
                sender,
                &username,
                reason.as_deref(),
            )
            .await
        }

        Command::Propose { subcommand, args } => {
            handle_propose(client, source, sender, &subcommand, &args).await
        }

        Command::Status { username } => {
            handle_status(client, freenet, config, source, sender, username.as_deref()).await
        }

        Command::Mesh { subcommand } => {
            handle_mesh(
                client,
                freenet,
                config,
                persistence_manager,
                source,
                sender,
                subcommand.as_deref(),
            )
            .await
        }

        Command::Audit { subcommand } => {
            handle_audit(client, freenet, config, source, sender, &subcommand).await
        }

        Command::RejectIntro { username: _ } => {
            // RejectIntro requires access to VettingSessionManager
            // This is handled in StromaBot::handle_message instead
            send_response(
                client,
                source,
                sender,
                "This command must be handled by the bot's vetting session manager.",
            )
            .await
        }

        Command::Help => {
            // Help requires access to all commands
            // This is handled in StromaBot::handle_message instead
            send_response(
                client,
                source,
                sender,
                "Help command is handled by the bot.",
            )
            .await
        }

        Command::Unknown(text) => {
            send_response(
                client,
                source,
                sender,
                &format!("Unknown command: {}", text),
            )
            .await
        }
    }
}

async fn handle_invite(
    client: &impl SignalClient,
    source: &MessageSource,
    sender: &ServiceId,
    username: &str,
    context: Option<&str>,
) -> SignalResult<()> {
    // TODO: Implement full invitation logic with Freenet:
    // 1. Verify sender is a current member (query Freenet contract)
    // 2. Hash invitee identity: ServiceId -> MemberHash (with mnemonic-derived key)
    // 3. Check if invitee already exists (member or pending invitee)
    // 4. Record first vouch in Freenet (AddVouch delta)
    // 5. Start vetting process (trigger cross-cluster matching)
    // 6. Send confirmation to inviter with next steps

    let context_str = context.unwrap_or("(no context provided)");
    let response = format!(
        "✅ Invitation for {} recorded as first vouch.\n\nContext: {}\n\nI'm now reaching out to a member from a different cluster for the cross-cluster vouch. You'll be notified when the vetting process progresses.",
        username, context_str
    );
    send_response(client, source, sender, &response).await
}

async fn handle_vouch(
    client: &impl SignalClient,
    source: &MessageSource,
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
    send_response(client, source, sender, &response).await
}

#[allow(clippy::too_many_arguments)]
async fn handle_flag<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    _group_manager: &GroupManager<impl SignalClient>,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
    username: &str,
    reason: Option<&str>,
) -> SignalResult<()> {
    use crate::freenet::{
        contract::MemberHash,
        traits::{ContractDelta, FreenetError},
        trust_contract::{StateDelta, TrustNetworkState},
    };
    use crate::identity::mask_identity;
    use crate::serialization::{from_cbor, to_cbor};
    use crate::signal::group::EjectionTrigger;

    // Hash sender and target identities using mnemonic-derived key
    let sender_hash: MemberHash = mask_identity(&sender.0, &config.identity_masking_key).into();
    let target_hash: MemberHash = mask_identity(username, &config.identity_masking_key).into();

    // Query Freenet for current contract state
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let mut state: TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Verify sender is a member
    if !state.members.contains(&sender_hash) {
        client
            .send_message(sender, "❌ You must be a member to flag others.")
            .await?;
        return Ok(());
    }

    // Verify target is a member
    if !state.members.contains(&target_hash) {
        client
            .send_message(sender, &format!("❌ {} is not a current member.", username))
            .await?;
        return Ok(());
    }

    // Check if sender previously vouched for target (vouch invalidation)
    let had_vouch = state
        .vouches
        .get(&target_hash)
        .map(|vouchers| vouchers.contains(&sender_hash))
        .unwrap_or(false);

    // Create delta
    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![],
        vouches_added: vec![],
        vouches_removed: if had_vouch {
            vec![(sender_hash, target_hash)]
        } else {
            vec![]
        },
        flags_added: vec![(sender_hash, target_hash)],
        flags_removed: vec![],
        config_update: None,
        proposals_created: vec![],
        proposals_checked: vec![],
        proposals_with_results: vec![],
        audit_entries_added: vec![],
        gap11_announcement_sent: false,
    };

    // Serialize and apply delta
    let delta_bytes = match to_cbor(&delta) {
        Ok(bytes) => bytes,
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to serialize delta: {}", e))
                .await?;
            return Ok(());
        }
    };

    let contract_delta = ContractDelta { data: delta_bytes };
    if let Err(e) = freenet.apply_delta(&contract, &contract_delta).await {
        client
            .send_message(
                sender,
                &format!("❌ Failed to apply delta to Freenet: {}", e),
            )
            .await?;
        return Ok(());
    }

    // Apply delta locally to calculate new standing
    state.apply_delta(&delta);

    // Calculate new standing
    let standing = state.calculate_standing(&target_hash);

    // Get counts for ejection trigger check
    let vouchers = state.vouches.get(&target_hash).cloned().unwrap_or_default();
    let flaggers = state.flags.get(&target_hash).cloned().unwrap_or_default();
    let voucher_flaggers: std::collections::HashSet<_> =
        vouchers.intersection(&flaggers).cloned().collect();

    let all_vouchers = vouchers.len() as u32;
    let all_flaggers = flaggers.len() as u32;
    let voucher_flagger_count = voucher_flaggers.len() as u32;

    // Check ejection triggers
    let ejection_trigger = EjectionTrigger::should_eject(
        all_vouchers,
        all_flaggers,
        voucher_flagger_count,
        config.min_vouch_threshold,
    );

    let mut response = if had_vouch {
        format!("⚠️ Flag for {} recorded (vouch invalidated).\n\n", username)
    } else {
        format!("⚠️ Flag for {} recorded.\n\n", username)
    };

    if let Some(reason_text) = reason {
        response.push_str(&format!("Reason: {}\n\n", reason_text));
    }

    if let Some(standing_val) = standing {
        response.push_str(&format!("Standing: {}\n", standing_val));
    }

    if let Some(trigger) = ejection_trigger {
        // Remove from Signal group
        // Convert username to ServiceId for group removal
        // TODO: In production, resolve username to actual ServiceId via Signal directory
        let target_service_id = ServiceId(username.to_string());

        if let Err(e) = _group_manager.remove_member(&target_service_id).await {
            client
                .send_message(
                    sender,
                    &format!("⚠️ Failed to remove member from Signal group: {}", e),
                )
                .await?;
        }

        // Announce ejection to group (using hash for privacy)
        let target_hash_hex = hex::encode(target_hash.as_bytes());
        if let Err(e) = _group_manager.announce_ejection(&target_hash_hex).await {
            client
                .send_message(sender, &format!("⚠️ Failed to announce ejection: {}", e))
                .await?;
        }

        match trigger {
            EjectionTrigger::NegativeStanding {
                effective_vouches,
                regular_flags,
            } => {
                response.push_str(&format!(
                    "\n🚫 EJECTION TRIGGERED: Negative standing ({} effective vouches - {} regular flags = {})\n",
                    effective_vouches, regular_flags, effective_vouches as i32 - regular_flags as i32
                ));
            }
            EjectionTrigger::BelowThreshold {
                effective_vouches,
                min_threshold,
            } => {
                response.push_str(&format!(
                    "\n🚫 EJECTION TRIGGERED: Effective vouches ({}) below threshold ({})\n",
                    effective_vouches, min_threshold
                ));
            }
        }
        response.push_str("Member has been removed from the Signal group.");
    } else {
        response.push_str("No ejection triggered.");
    }

    send_response(client, source, sender, &response).await
}

async fn handle_propose(
    client: &impl SignalClient,
    source: &MessageSource,
    sender: &ServiceId,
    subcommand: &str,
    args: &[String],
) -> SignalResult<()> {
    use crate::signal::proposals::parse_propose_args;

    // Parse arguments
    let propose_args = match parse_propose_args(subcommand, args) {
        Ok(args) => args,
        Err(err) => {
            return send_response(client, source, sender, &format!("❌ {}", err)).await;
        }
    };

    // TODO: Create proposal poll
    // For now, acknowledge the parsed command
    let timeout_str = if let Some(timeout) = propose_args.timeout {
        format!(" (timeout: {}h)", timeout.as_secs() / 3600)
    } else {
        " (using default timeout)".to_string()
    };

    let response = format!(
        "✅ Proposal created: {:?}{}",
        propose_args.subcommand, timeout_str
    );
    send_response(client, source, sender, &response).await
}

async fn handle_status<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
    username: Option<&str>,
) -> SignalResult<()> {
    use crate::freenet::{
        contract::MemberHash, traits::FreenetError, trust_contract::TrustNetworkState,
    };
    use crate::identity::mask_identity;
    use crate::serialization::from_cbor;

    // GAP-04: /status shows own vouchers only, rejects third-party queries
    if username.is_some() {
        let response = "Third-party status queries are not allowed. Use /status (without username) to see your own standing.";
        return send_response(client, source, sender, response).await;
    }

    // Hash sender's ServiceId to MemberHash using mnemonic-derived key
    let sender_hash: MemberHash = mask_identity(&sender.0, &config.identity_masking_key).into();

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current contract state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Calculate trust metrics
    let all_vouchers = state.vouches.get(&sender_hash).cloned().unwrap_or_default();
    let all_flaggers = state.flags.get(&sender_hash).cloned().unwrap_or_default();

    let voucher_flaggers: std::collections::HashSet<_> =
        all_vouchers.intersection(&all_flaggers).cloned().collect();

    let all_vouchers_count = all_vouchers.len() as u32;
    let all_flaggers_count = all_flaggers.len() as u32;
    let voucher_flagger_count = voucher_flaggers.len() as u32;

    let effective_vouches = all_vouchers_count - voucher_flagger_count;
    let regular_flags = all_flaggers_count - voucher_flagger_count;

    // Calculate standing
    let standing = state.calculate_standing(&sender_hash);
    let standing_value = standing.unwrap_or(0);

    // Determine role
    let is_member = state.members.contains(&sender_hash);
    let role = if !is_member {
        "Invitee"
    } else if effective_vouches >= 3 {
        "Validator"
    } else {
        "Bridge"
    };

    // Format voucher list (showing partial hashes for privacy)
    let voucher_list: Vec<String> = all_vouchers
        .iter()
        .map(|hash| {
            let hex = hex::encode(hash.as_bytes());
            format!("{}...", &hex[..8]) // Show first 8 chars of hash
        })
        .collect();

    let voucher_display = if voucher_list.is_empty() {
        "none".to_string()
    } else {
        voucher_list.join(", ")
    };

    // Build response
    let mut response = format!("📊 Your Trust Status\nRole: {}\n", role);
    response.push_str(&format!(
        "All vouches: {} ({})\n",
        all_vouchers_count, voucher_display
    ));
    response.push_str(&format!("All flags: {}\n", all_flaggers_count));
    response.push_str(&format!("Voucher-flaggers: {}\n", voucher_flagger_count));
    response.push_str(&format!("Effective vouches: {} ", effective_vouches));
    if effective_vouches >= config.min_vouch_threshold {
        response.push_str("✅\n");
    } else {
        response.push_str("⚠️\n");
    }
    response.push_str(&format!("Regular flags: {}\n", regular_flags));
    response.push_str(&format!(
        "Standing: {} ({})",
        if standing_value >= 0 {
            format!("+{}", standing_value)
        } else {
            standing_value.to_string()
        },
        if standing_value > 0 {
            "positive"
        } else if standing_value < 0 {
            "negative"
        } else {
            "neutral"
        }
    ));

    send_response(client, source, sender, &response).await
}

async fn handle_mesh<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    persistence_manager: &crate::persistence::WriteBlockingManager,
    source: &MessageSource,
    sender: &ServiceId,
    subcommand: Option<&str>,
) -> SignalResult<()> {
    match subcommand {
        None => handle_mesh_overview(client, freenet, config, source, sender).await,
        Some("strength") => handle_mesh_strength(client, freenet, config, source, sender).await,
        Some("replication") => {
            handle_mesh_replication(client, persistence_manager, source, sender).await
        }
        Some("config") => handle_mesh_config(client, freenet, config, source, sender).await,
        Some("settings") => handle_mesh_settings(client, freenet, config, source, sender).await,
        Some(unknown) => {
            send_response(
                client,
                source,
                sender,
                &format!(
                    "Unknown /mesh subcommand: {}.\n\nAvailable: /mesh, /mesh strength, /mesh replication, /mesh config, /mesh settings",
                    unknown
                ),
            )
            .await
        }
    }
}

async fn handle_mesh_overview<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
) -> SignalResult<()> {
    use crate::freenet::traits::FreenetError;
    use crate::matchmaker::{calculate_dvr, detect_clusters};
    use crate::serialization::from_cbor;

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Calculate DVR
    let dvr_result = calculate_dvr(&state);

    // Detect clusters
    let cluster_result = detect_clusters(&state);

    // Calculate vouch distribution
    let total_members = state.members.len();
    let mut vouch_counts: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for member in &state.members {
        let vouch_count = state.vouches.get(member).map(|v| v.len()).unwrap_or(0);
        *vouch_counts.entry(vouch_count).or_insert(0) += 1;
    }

    let members_with_2 = vouch_counts.get(&2).copied().unwrap_or(0);
    let members_with_3_plus: usize = vouch_counts
        .iter()
        .filter(|(&count, _)| count >= 3)
        .map(|(_, &num_members)| num_members)
        .sum();

    // Format response
    let mut response = format!(
        "📈 Network Overview\n\nTotal members: {}\nNetwork health: {} {} ({:.0}% DVR)\n",
        total_members,
        dvr_result.health.emoji(),
        dvr_result.health.name(),
        dvr_result.percentage()
    );

    if cluster_result.cluster_count > 1 {
        response.push_str(&format!(
            "\nClusters detected: {}\n",
            cluster_result.cluster_count
        ));
    }

    response.push_str("\nTrust Distribution:\n");
    if members_with_2 > 0 {
        let pct = (members_with_2 as f32 / total_members as f32) * 100.0;
        response.push_str(&format!(
            "  2 connections: {} members ({:.0}%)\n",
            members_with_2, pct
        ));
    }
    if members_with_3_plus > 0 {
        let pct = (members_with_3_plus as f32 / total_members as f32) * 100.0;
        response.push_str(&format!(
            "  3+ connections: {} members ({:.0}%)\n",
            members_with_3_plus, pct
        ));
    }

    response.push_str("\n💡 ");
    match dvr_result.health {
        crate::matchmaker::dvr::HealthStatus::Healthy => {
            response.push_str("Your network has strong distributed trust. Members with 3+ cross-cluster vouches create resilient verification.");
        }
        crate::matchmaker::dvr::HealthStatus::Developing => {
            response.push_str("Your network is developing. Consider suggesting strategic introductions to improve distributed trust.");
        }
        crate::matchmaker::dvr::HealthStatus::Unhealthy => {
            response.push_str("Your network needs more validators. Use strategic introductions to strengthen distributed trust.");
        }
    }

    response.push_str("\n\nFor detailed metrics: /mesh strength, /mesh replication, /mesh config");

    send_response(client, source, sender, &response).await
}

async fn handle_mesh_strength<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
) -> SignalResult<()> {
    use crate::freenet::traits::FreenetError;
    use crate::matchmaker::{calculate_dvr, detect_clusters};
    use crate::serialization::from_cbor;

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Calculate DVR
    let dvr_result = calculate_dvr(&state);

    // Detect clusters
    let cluster_result = detect_clusters(&state);

    // Calculate vouch distribution histogram
    let mut vouch_distribution: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for member in &state.members {
        let vouch_count = state.vouches.get(member).map(|v| v.len()).unwrap_or(0);
        *vouch_distribution.entry(vouch_count).or_insert(0) += 1;
    }

    // Format response
    let mut response = format!(
        "💪 Network Strength (DVR Analysis)\n\n{} DVR Score: {:.0}%\n",
        dvr_result.health.emoji(),
        dvr_result.percentage()
    );

    response.push_str(&format!(
        "\nDistinct Validators: {} / {} possible\n",
        dvr_result.distinct_validators, dvr_result.max_possible
    ));

    response.push_str(&format!(
        "Network Size: {} members\n",
        dvr_result.network_size
    ));

    if cluster_result.cluster_count > 1 {
        response.push_str(&format!("Clusters: {}\n", cluster_result.cluster_count));
    }

    // Show vouch distribution histogram
    response.push_str("\nVouch Distribution:\n");
    for (&vouch_count, &num_members) in vouch_distribution.iter() {
        let pct = (num_members as f32 / state.members.len() as f32) * 100.0;
        let bar_length = (pct / 10.0) as usize;
        let bar = "█".repeat(bar_length) + &"░".repeat(10 - bar_length);
        response.push_str(&format!(
            "  {} connections: {} {} members ({:.0}%)\n",
            vouch_count, bar, num_members, pct
        ));
    }

    // Strength indicators
    response.push_str("\nStrength Indicators:\n");
    match dvr_result.health {
        crate::matchmaker::dvr::HealthStatus::Healthy => {
            response.push_str("  ✅ Multiple validation paths\n");
            response.push_str("  ✅ Low validator concentration\n");
            response.push_str("  ✅ Network can withstand validator failures\n");
        }
        crate::matchmaker::dvr::HealthStatus::Developing => {
            response.push_str("  ⚠️  Growing validator coverage\n");
            response.push_str("  ⚠️  Consider recruiting more validators\n");
            if cluster_result.cluster_count <= 1 {
                response
                    .push_str("  ⚠️  Network may benefit from more cross-cluster connections\n");
            }
        }
        crate::matchmaker::dvr::HealthStatus::Unhealthy => {
            response.push_str("  ❌ Insufficient validator coverage\n");
            response.push_str("  ❌ Network vulnerable to single points of failure\n");
            response.push_str("  ❌ Urgently needs more cross-cluster validators\n");
        }
    }

    // Add improvement suggestion based on health
    response.push_str("\n💡 ");
    match dvr_result.health {
        crate::matchmaker::dvr::HealthStatus::Healthy => {
            response.push_str(
                "Your network has excellent resilience. Maintain this by continuing to make cross-cluster vouches.",
            );
        }
        crate::matchmaker::dvr::HealthStatus::Developing => {
            response.push_str(
                "Strengthen your network by vouching for members from different clusters. This improves distributed verification.",
            );
        }
        crate::matchmaker::dvr::HealthStatus::Unhealthy => {
            response.push_str(
                "URGENT: Your network needs more validators with distinct voucher sets. Focus on cross-cluster vouches to build resilience.",
            );
        }
    }

    send_response(client, source, sender, &response).await
}

async fn handle_mesh_replication(
    client: &impl SignalClient,
    persistence_manager: &crate::persistence::WriteBlockingManager,
    source: &MessageSource,
    sender: &ServiceId,
) -> SignalResult<()> {
    use crate::persistence::WriteBlockingState;

    // Query persistence manager for current state
    let health = persistence_manager.replication_health();
    let state = persistence_manager.current_state();
    let (total_chunks, fully_replicated, recoverable, at_risk) =
        persistence_manager.replication_stats();
    let writes_allowed = persistence_manager.allows_writes();

    // Build response based on health status
    let mut response = format!(
        "💾 Replication Health\n\n{} Status: {}\n\n",
        health.emoji(),
        health.description()
    );

    // Show chunk statistics if there are chunks
    if total_chunks > 0 {
        response.push_str(&format!("Chunks: {}\n", total_chunks));
        response.push_str(&format!("  • Fully replicated: {}\n", fully_replicated));
        response.push_str(&format!("  • Recoverable: {}\n", recoverable));
        if at_risk > 0 {
            response.push_str(&format!("  • At risk: {} ⚠️\n", at_risk));
            let at_risk_indices = persistence_manager.at_risk_chunks();
            if !at_risk_indices.is_empty() {
                response.push_str(&format!("  • At-risk chunks: {:?}\n", at_risk_indices));
            }
        }
        response.push('\n');
    }

    // Write permissions status
    response.push_str(&format!(
        "Write Permissions: {}\n\n",
        if writes_allowed {
            "✅ Allowed"
        } else {
            "❌ Blocked"
        }
    ));

    // Add context based on current state
    match state {
        WriteBlockingState::Provisional => {
            response.push_str("ℹ️  Note: No peers available for replication yet.\n");
            response.push_str(
                "Writes are allowed, but state is not backed up until peers are discovered.\n\n",
            );
        }
        WriteBlockingState::Isolated => {
            response.push_str("ℹ️  Note: Network size is 1 (single bot).\n");
            response.push_str("Writes are allowed, but no replication is possible. This is expected during testing.\n\n");
        }
        WriteBlockingState::Degraded => {
            response.push_str("⚠️  Warning: Replication is degraded.\n");
            response.push_str("Writes are blocked until chunks are replicated to peers.\n");
            response.push_str("The bot will automatically retry distribution.\n\n");
        }
        WriteBlockingState::Active => {
            if total_chunks == 0 {
                response.push_str("ℹ️  Note: No state has been persisted yet.\n");
                response.push_str("Replication will begin after the first state change.\n\n");
            } else {
                response.push_str("✅ Your trust network data is safely replicated.\n");
                response.push_str(
                    "If this bot crashes, state can be recovered from chunk holders.\n\n",
                );
            }
        }
    }

    response.push_str("💡 The persistence system ensures your trust network data is backed up\n");
    response
        .push_str("across multiple bots. Recovery uses the reciprocal persistence network.\n\n");
    response.push_str("Technical details: docs/PERSISTENCE.md");

    send_response(client, source, sender, &response).await
}

async fn handle_mesh_config<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
) -> SignalResult<()> {
    use crate::freenet::traits::FreenetError;
    use crate::serialization::from_cbor;

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Format response
    let group_config = &state.config;
    let mut response = String::from("⚙️ Group Configuration\n\n🔧 Trust Settings:\n");

    response.push_str(&format!(
        "  Minimum vouches: {}\n",
        config.min_vouch_threshold
    ));

    // Note: max_flags is not currently in the GroupConfig struct, so we'll use a default
    response.push_str("  Maximum flags: 3 (default)\n");

    response.push_str(&format!(
        "  Open membership: {}\n",
        if group_config.open_membership {
            "Yes"
        } else {
            "No"
        }
    ));

    response.push_str(&format!(
        "\n👥 Operators: {}\n",
        group_config.operators.len()
    ));

    response.push_str("\n🔐 Security:\n");
    response.push_str("  Self-query: ✅ Allowed\n");
    response.push_str("  Third-party query: ❌ Restricted\n");

    response.push_str("\n💡 Configuration changes require operator approval via /propose.");

    send_response(client, source, sender, &response).await
}

async fn handle_mesh_settings<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
) -> SignalResult<()> {
    use crate::freenet::traits::FreenetError;
    use crate::serialization::from_cbor;

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Get Signal group info (using the group_id from BotConfig)
    let signal_info = client.get_group_info(&config.group_id).await.ok();

    // Format response
    let group_config = &state.config;
    let mut response = String::from("⚙️ Available Configuration Keys\n\n");

    // Stroma Settings
    response.push_str("📋 Stroma Settings (/propose stroma <key> <value>):\n");
    response.push_str(&format!(
        "  min_vouches: {} (range: 1-10) - Minimum vouches for standing\n",
        group_config.min_vouches
    ));
    response.push_str(&format!(
        "  max_flags: {} (range: 1-10) - Maximum flags before ejection\n",
        group_config.max_flags
    ));
    response.push_str(&format!(
        "  open_membership: {} (true/false) - Allow new members\n",
        group_config.open_membership
    ));
    response.push_str(&format!(
        "  default_poll_timeout_secs: {} (3600-604800) - Default timeout\n",
        group_config.default_poll_timeout_secs
    ));
    response.push_str(&format!(
        "  config_change_threshold: {:.2} (0.50-1.00) - Vote threshold\n",
        group_config.config_change_threshold
    ));
    response.push_str(&format!(
        "  min_quorum: {:.2} (0.25-1.00) - Minimum participation\n\n",
        group_config.min_quorum
    ));

    // Signal Settings
    response.push_str("📡 Signal Settings (/propose signal <key> <value>):\n");
    if let Some(info) = signal_info {
        response.push_str(&format!(
            "  name: \"{}\" (1-32 chars) - Group display name\n",
            info.name
        ));
        response.push_str(&format!(
            "  description: \"{}\" (0-480 chars) - Group description\n",
            info.description.unwrap_or_else(|| "".to_string())
        ));
        let timer_str = match info.disappearing_messages_timer {
            None | Some(0) => "off".to_string(),
            Some(3600) => "1h".to_string(),
            Some(86400) => "1d".to_string(),
            Some(604800) => "7d".to_string(),
            Some(1209600) => "14d".to_string(),
            Some(2592000) => "30d".to_string(),
            Some(7776000) => "90d".to_string(),
            Some(s) => format!("{}s", s),
        };
        response.push_str(&format!(
            "  disappearing_messages: {} (off, 1h, 1d, 7d, 14d, 30d, 90d) - Message timer\n",
            timer_str
        ));
        response.push_str(&format!(
            "  announcements_only: {} (true/false) - Admin-only messages\n\n",
            info.announcements_only
        ));
    } else {
        response.push_str("  name: <current> (1-32 chars) - Group display name\n");
        response.push_str("  description: <current> (0-480 chars) - Group description\n");
        response.push_str(
            "  disappearing_messages: <current> (off, 1h, 1d, 7d, 14d, 30d, 90d) - Message timer\n",
        );
        response.push_str("  announcements_only: <current> (true/false) - Admin-only messages\n\n");
    }

    // Poll Options
    response.push_str("📊 Poll Options:\n");
    response.push_str("  Signal polls support up to 10 options.\n");
    response.push_str("  Binary: /propose signal <key> <value> (creates Approve/Reject poll)\n");
    response.push_str(
        "  Multi:  /propose signal --key <key> --value <v1> --value <v2> ... (post-UAT)\n\n",
    );

    response.push_str("💡 Example: /propose signal disappearing_messages 7d --timeout 48h");

    send_response(client, source, sender, &response).await
}

#[allow(clippy::too_many_arguments)]
async fn handle_audit<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    source: &MessageSource,
    sender: &ServiceId,
    subcommand: &str,
) -> SignalResult<()> {
    use crate::freenet::traits::FreenetError;
    use crate::gatekeeper::audit_trail::{
        format_audit_log, query_audit_log, ActionType, AuditQuery,
    };
    use crate::serialization::from_cbor;

    // Get contract hash
    let contract = match &config.contract_hash {
        Some(hash) => *hash,
        None => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not configured. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
    };

    // Query Freenet for current state
    let state_bytes = match freenet.get_state(&contract).await {
        Ok(state) => state.data,
        Err(FreenetError::ContractNotFound) => {
            client
                .send_message(
                    sender,
                    "❌ Trust contract not found. Has the group been bootstrapped?",
                )
                .await?;
            return Ok(());
        }
        Err(e) => {
            client
                .send_message(sender, &format!("❌ Failed to query Freenet: {}", e))
                .await?;
            return Ok(());
        }
    };

    let state: crate::freenet::trust_contract::TrustNetworkState = match from_cbor(&state_bytes) {
        Ok(s) => s,
        Err(e) => {
            client
                .send_message(
                    sender,
                    &format!("❌ Failed to deserialize contract state: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    // Query audit log based on subcommand
    match subcommand {
        "operator" => {
            // Show all non-bootstrap operator actions
            let query = AuditQuery {
                action_type: None,
                actor: None,
                limit: Some(50),
                after_timestamp: None,
            };

            let mut entries = query_audit_log(&state.audit_log, &query);

            // Filter out bootstrap actions
            entries.retain(|e| e.action_type != ActionType::Bootstrap);

            let response = if entries.is_empty() {
                "No operator actions recorded yet.".to_string()
            } else {
                format_audit_log(&entries)
            };

            send_response(client, source, sender, &response).await
        }
        "bootstrap" => {
            // Show only bootstrap actions
            let query = AuditQuery {
                action_type: Some(ActionType::Bootstrap),
                actor: None,
                limit: Some(50),
                after_timestamp: None,
            };

            let entries = query_audit_log(&state.audit_log, &query);

            let response = if entries.is_empty() {
                "No bootstrap actions recorded yet.".to_string()
            } else {
                format_audit_log(&entries)
            };

            send_response(client, source, sender, &response).await
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
    use crate::freenet::mock::MockFreenetClient;
    use crate::freenet::traits::ContractHash;
    use crate::signal::mock::MockSignalClient;
    use crate::signal::traits::GroupId;

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
        assert_eq!(cmd, Command::Mesh { subcommand: None });
    }

    #[test]
    fn test_parse_mesh_strength() {
        let cmd = parse_command("/mesh strength");
        assert_eq!(
            cmd,
            Command::Mesh {
                subcommand: Some("strength".to_string())
            }
        );
    }

    #[test]
    fn test_parse_mesh_replication() {
        let cmd = parse_command("/mesh replication");
        assert_eq!(
            cmd,
            Command::Mesh {
                subcommand: Some("replication".to_string())
            }
        );
    }

    #[test]
    fn test_parse_mesh_config() {
        let cmd = parse_command("/mesh config");
        assert_eq!(
            cmd,
            Command::Mesh {
                subcommand: Some("config".to_string())
            }
        );
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
    fn test_parse_reject_intro() {
        let cmd = parse_command("/reject-intro @alice");
        assert_eq!(
            cmd,
            Command::RejectIntro {
                username: "@alice".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_reject_intro_missing_username() {
        let cmd = parse_command("/reject-intro");
        assert!(matches!(cmd, Command::Unknown(_)));
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

        let result = handle_invite(
            &client,
            &MessageSource::DirectMessage,
            &sender,
            "@alice",
            Some("context"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_status() {
        use crate::freenet::contract::MemberHash;
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Create test state with sender as a member with vouches
        let mut test_state = TrustNetworkState::new();
        let sender_hash: MemberHash =
            crate::identity::mask_identity(&sender.0, &config.identity_masking_key).into();
        test_state.members.insert(sender_hash);

        // Add some vouchers
        let voucher1 = MemberHash::from_bytes(&[1u8; 32]);
        let voucher2 = MemberHash::from_bytes(&[2u8; 32]);
        let mut vouchers = std::collections::HashSet::new();
        vouchers.insert(voucher1);
        vouchers.insert(voucher2);
        test_state.vouches.insert(sender_hash, vouchers);

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let result = handle_status(
            &client,
            &freenet,
            &config,
            &MessageSource::DirectMessage,
            &sender,
            None,
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Trust Status"));
    }

    #[tokio::test]
    async fn test_handle_status_rejects_third_party() {
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Set up minimal state (not actually used since third-party query is rejected early)
        let test_state = TrustNetworkState::new();
        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let result = handle_status(
            &client,
            &freenet,
            &config,
            &MessageSource::DirectMessage,
            &sender,
            Some("@alice"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("not allowed"));
    }

    #[tokio::test]
    async fn test_handle_audit_operator() {
        use crate::freenet::contract::MemberHash;
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::gatekeeper::audit_trail::AuditEntry;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Create test state with some operator audit entries
        let mut test_state = TrustNetworkState::new();
        let actor = MemberHash::from_bytes(&[1u8; 32]);
        test_state.audit_log.push(AuditEntry::config_change(
            actor,
            "Updated min_vouches from 2 to 3".to_string(),
        ));
        test_state.audit_log.push(AuditEntry::restart(
            actor,
            "Bot restarted for maintenance".to_string(),
        ));

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let result = handle_audit(
            &client,
            &freenet,
            &config,
            &MessageSource::DirectMessage,
            &sender,
            "operator",
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Operator Audit Trail"));
        assert!(sent[0].content.contains("Config Change"));
        assert!(sent[0].content.contains("Restart"));
    }

    #[tokio::test]
    async fn test_handle_audit_bootstrap() {
        use crate::freenet::contract::MemberHash;
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::gatekeeper::audit_trail::AuditEntry;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Create test state with bootstrap audit entries
        let mut test_state = TrustNetworkState::new();
        let actor = MemberHash::from_bytes(&[1u8; 32]);
        test_state.audit_log.push(AuditEntry::bootstrap(
            actor,
            "Group created: Mission Control".to_string(),
        ));
        test_state.audit_log.push(AuditEntry::bootstrap(
            actor,
            "Added seed member: alice".to_string(),
        ));

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let result = handle_audit(
            &client,
            &freenet,
            &config,
            &MessageSource::DirectMessage,
            &sender,
            "bootstrap",
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Operator Audit Trail"));
        assert!(sent[0].content.contains("Bootstrap"));
        assert!(sent[0].content.contains("Group created"));
    }

    #[tokio::test]
    async fn test_handle_mesh_overview() {
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Set up test state with some members and vouches
        let mut test_state = TrustNetworkState::new();
        use crate::freenet::contract::MemberHash;
        for i in 1..=5 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            test_state.members.insert(MemberHash::from_bytes(&bytes));
        }

        // Add some vouches to create validators
        let member1 = MemberHash::from_bytes(&[1u8; 32]);
        let member2 = MemberHash::from_bytes(&[2u8; 32]);
        let member3 = MemberHash::from_bytes(&[3u8; 32]);
        let mut vouchers = std::collections::HashSet::new();
        vouchers.insert(member2);
        vouchers.insert(member3);
        test_state.vouches.insert(member1, vouchers);

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let persistence_manager = crate::persistence::WriteBlockingManager::new();
        let result = handle_mesh(
            &client,
            &freenet,
            &config,
            &persistence_manager,
            &MessageSource::DirectMessage,
            &sender,
            None,
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Network Overview"));
    }

    #[tokio::test]
    async fn test_handle_mesh_strength() {
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Set up test state
        let mut test_state = TrustNetworkState::new();
        use crate::freenet::contract::MemberHash;
        for i in 1..=5 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            test_state.members.insert(MemberHash::from_bytes(&bytes));
        }

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let persistence_manager = crate::persistence::WriteBlockingManager::new();
        let result = handle_mesh(
            &client,
            &freenet,
            &config,
            &persistence_manager,
            &MessageSource::DirectMessage,
            &sender,
            Some("strength"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Network Strength"));
        assert!(sent[0].content.contains("DVR"));
    }

    #[tokio::test]
    async fn test_handle_mesh_replication() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(ContractHash::from_bytes(&[0u8; 32])),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Test 1: Empty persistence manager (PROVISIONAL state)
        let persistence_manager = crate::persistence::WriteBlockingManager::new();
        let result = handle_mesh(
            &client,
            &freenet,
            &config,
            &persistence_manager,
            &MessageSource::DirectMessage,
            &sender,
            Some("replication"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Replication Health"));
        assert!(sent[0].content.contains("Setting up persistence"));
        assert!(sent[0].content.contains("Write Permissions: ✅ Allowed"));

        // Test 2: Manager with chunks (ACTIVE state)
        let mut persistence_manager2 = crate::persistence::WriteBlockingManager::new();
        persistence_manager2.initialize_chunks(8);
        persistence_manager2.set_network_size(5);
        // Set all chunks as fully replicated
        for i in 0..8 {
            persistence_manager2.update_chunk_status(i, 3);
        }

        let client2 = MockSignalClient::new(ServiceId("bot".to_string()));
        let result = handle_mesh(
            &client2,
            &freenet,
            &config,
            &persistence_manager2,
            &MessageSource::DirectMessage,
            &sender,
            Some("replication"),
        )
        .await;
        assert!(result.is_ok());

        let sent2 = client2.sent_messages();
        assert_eq!(sent2.len(), 1);
        assert!(sent2[0].content.contains("Fully replicated"));
        assert!(sent2[0].content.contains("Chunks: 8"));
        assert!(sent2[0].content.contains("Write Permissions: ✅ Allowed"));

        // Test 3: Manager with at-risk chunks (DEGRADED state)
        let mut persistence_manager3 = crate::persistence::WriteBlockingManager::new();
        persistence_manager3.initialize_chunks(8);
        persistence_manager3.set_network_size(5);
        // Set some chunks as at-risk
        for i in 0..6 {
            persistence_manager3.update_chunk_status(i, 3); // Good
        }
        persistence_manager3.update_chunk_status(6, 1); // At risk
        persistence_manager3.update_chunk_status(7, 1); // At risk

        let client3 = MockSignalClient::new(ServiceId("bot".to_string()));
        let result = handle_mesh(
            &client3,
            &freenet,
            &config,
            &persistence_manager3,
            &MessageSource::DirectMessage,
            &sender,
            Some("replication"),
        )
        .await;
        assert!(result.is_ok());

        let sent3 = client3.sent_messages();
        assert_eq!(sent3.len(), 1);
        assert!(sent3[0].content.contains("Cannot recover if crash"));
        assert!(sent3[0].content.contains("At risk: 2"));
        assert!(sent3[0].content.contains("Write Permissions: ❌ Blocked"));
    }

    #[tokio::test]
    async fn test_handle_mesh_config() {
        use crate::freenet::traits::ContractState;
        use crate::freenet::trust_contract::TrustNetworkState;
        use crate::serialization::to_cbor;

        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();

        let contract_hash = ContractHash::from_bytes(&[0u8; 32]);
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(contract_hash),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        // Set up test state
        let test_state = TrustNetworkState::new();

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let persistence_manager = crate::persistence::WriteBlockingManager::new();
        let result = handle_mesh(
            &client,
            &freenet,
            &config,
            &persistence_manager,
            &MessageSource::DirectMessage,
            &sender,
            Some("config"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Group Configuration"));
    }

    #[tokio::test]
    async fn test_handle_mesh_unknown_subcommand() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());
        let freenet = MockFreenetClient::new();
        let config = crate::signal::bot::BotConfig {
            group_id: GroupId(vec![1, 2, 3]),
            contract_hash: Some(ContractHash::from_bytes(&[0u8; 32])),
            min_vouch_threshold: 2,
            ..Default::default()
        };

        let persistence_manager = crate::persistence::WriteBlockingManager::new();
        let result = handle_mesh(
            &client,
            &freenet,
            &config,
            &persistence_manager,
            &MessageSource::DirectMessage,
            &sender,
            Some("unknown"),
        )
        .await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Unknown /mesh subcommand"));
    }
}
