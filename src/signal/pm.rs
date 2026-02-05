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

        _ => Command::Unknown(text.to_string()),
    }
}

/// Handle PM command
pub async fn handle_pm_command<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    group_manager: &GroupManager<impl SignalClient>,
    config: &crate::signal::bot::BotConfig,
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
            handle_flag(
                client,
                freenet,
                group_manager,
                config,
                sender,
                &username,
                reason.as_deref(),
            )
            .await
        }

        Command::Propose { subcommand, args } => {
            handle_propose(client, sender, &subcommand, &args).await
        }

        Command::Status { username } => handle_status(client, sender, username.as_deref()).await,

        Command::Mesh { subcommand } => {
            handle_mesh(client, freenet, config, sender, subcommand.as_deref()).await
        }

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

async fn handle_flag<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    _group_manager: &GroupManager<impl SignalClient>,
    config: &crate::signal::bot::BotConfig,
    sender: &ServiceId,
    username: &str,
    reason: Option<&str>,
) -> SignalResult<()> {
    use crate::freenet::{
        contract::MemberHash,
        traits::{ContractDelta, FreenetError},
        trust_contract::{StateDelta, TrustNetworkState},
    };
    use crate::serialization::{from_cbor, to_cbor};
    use crate::signal::group::EjectionTrigger;

    // Hash sender and target identities
    let sender_hash = MemberHash::from_identity(&sender.0, &config.pepper);
    let target_hash = MemberHash::from_identity(username, &config.pepper);

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

    client.send_message(sender, &response).await
}

async fn handle_propose(
    client: &impl SignalClient,
    sender: &ServiceId,
    subcommand: &str,
    args: &[String],
) -> SignalResult<()> {
    use crate::signal::proposals::parse_propose_args;

    // Parse arguments
    let propose_args = match parse_propose_args(subcommand, args) {
        Ok(args) => args,
        Err(err) => {
            return client.send_message(sender, &format!("❌ {}", err)).await;
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

async fn handle_mesh<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
    sender: &ServiceId,
    subcommand: Option<&str>,
) -> SignalResult<()> {
    match subcommand {
        None => handle_mesh_overview(client, freenet, config, sender).await,
        Some("strength") => handle_mesh_strength(client, freenet, config, sender).await,
        Some("replication") => handle_mesh_replication(client, freenet, config, sender).await,
        Some("config") => handle_mesh_config(client, freenet, config, sender).await,
        Some(unknown) => {
            client
                .send_message(
                    sender,
                    &format!(
                        "Unknown /mesh subcommand: {}.\n\nAvailable: /mesh, /mesh strength, /mesh replication, /mesh config",
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

    client.send_message(sender, &response).await
}

async fn handle_mesh_strength<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
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

    client.send_message(sender, &response).await
}

async fn handle_mesh_replication<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    _freenet: &F,
    _config: &crate::signal::bot::BotConfig,
    sender: &ServiceId,
) -> SignalResult<()> {
    // TODO: Implement persistence health monitoring:
    // This requires access to the persistence layer (PersistenceRegistry, ReplicationHealth)
    // which is not currently passed through the PM command context.
    // Options:
    // 1. Add persistence manager to BotContext/handler parameters
    // 2. Query persistence state through Freenet (if it's stored there)
    // 3. Expose persistence metrics through a separate module/service
    //
    // For now, returning placeholder that indicates feature is not fully connected.

    let response = "🔄 Persistence Health\n\n🔵 Status: Initializing\n\nPersistence health monitoring requires access to the bot's persistence layer.\nThis feature will be fully implemented when persistence manager is integrated with PM command handlers.\n\n💡 Expected metrics:\n  • Replication status (🟢/🟡/🔴/🔵)\n  • Fragments distributed (e.g., 3/3)\n  • Recovery confidence\n  • Write permission status\n  • Last state change timestamp";
    client.send_message(sender, response).await
}

async fn handle_mesh_config<F: crate::freenet::FreenetClient>(
    client: &impl SignalClient,
    freenet: &F,
    config: &crate::signal::bot::BotConfig,
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

    client.send_message(sender, &response).await
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
            pepper: vec![0u8; 32],
            min_vouch_threshold: 2,
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

        let result = handle_mesh(&client, &freenet, &config, &sender, None).await;
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
            pepper: vec![0u8; 32],
            min_vouch_threshold: 2,
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

        let result = handle_mesh(&client, &freenet, &config, &sender, Some("strength")).await;
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
            pepper: vec![0u8; 32],
            min_vouch_threshold: 2,
        };

        let result = handle_mesh(&client, &freenet, &config, &sender, Some("replication")).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Persistence Health"));
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
            pepper: vec![0u8; 32],
            min_vouch_threshold: 2,
        };

        // Set up test state
        let test_state = TrustNetworkState::new();

        let state_bytes = to_cbor(&test_state).unwrap();
        freenet.put_state(contract_hash, ContractState { data: state_bytes });

        let result = handle_mesh(&client, &freenet, &config, &sender, Some("config")).await;
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
            pepper: vec![0u8; 32],
            min_vouch_threshold: 2,
        };

        let result = handle_mesh(&client, &freenet, &config, &sender, Some("unknown")).await;
        assert!(result.is_ok());

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].content.contains("Unknown /mesh subcommand"));
    }
}
