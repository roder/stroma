//! Operator Audit Trail (GAP-01)
//!
//! Per TODO.md line 879:
//! - Log all operator actions (config changes, manual interventions)
//! - Fields: timestamp, actor (MemberHash), action type, action details
//! - Store in Freenet contract audit log
//! - Query interface for /audit operator command
//!
//! Design principles:
//! - Immutable append-only log (no deletion)
//! - Chronological ordering via timestamp
//! - Privacy: uses MemberHash, not cleartext identities
//! - Integration: stored in TrustNetworkState for Freenet persistence

use crate::freenet::contract::MemberHash;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Operator action types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    /// Configuration change (e.g., min_vouches, max_flags).
    ConfigChange,
    /// Bot restart or maintenance.
    Restart,
    /// Manual intervention (e.g., emergency ejection override).
    ManualIntervention,
    /// Bootstrap action (e.g., create group, add seed member).
    Bootstrap,
    /// Other operator action.
    Other(String),
}

/// Single audit log entry.
///
/// Per GAP-01: immutable, append-only, privacy-preserving.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unix timestamp (seconds since epoch).
    pub timestamp: u64,
    /// Actor who performed the action (operator MemberHash).
    pub actor: MemberHash,
    /// Type of action performed.
    pub action_type: ActionType,
    /// Human-readable action details (no sensitive data).
    pub details: String,
}

impl AuditEntry {
    /// Create a new audit entry with current timestamp.
    pub fn new(actor: MemberHash, action_type: ActionType, details: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before UNIX epoch")
            .as_secs();

        Self {
            timestamp,
            actor,
            action_type,
            details,
        }
    }

    /// Create entry for config change.
    pub fn config_change(actor: MemberHash, details: String) -> Self {
        Self::new(actor, ActionType::ConfigChange, details)
    }

    /// Create entry for restart.
    pub fn restart(actor: MemberHash, details: String) -> Self {
        Self::new(actor, ActionType::Restart, details)
    }

    /// Create entry for manual intervention.
    pub fn manual_intervention(actor: MemberHash, details: String) -> Self {
        Self::new(actor, ActionType::ManualIntervention, details)
    }

    /// Create entry for bootstrap action.
    pub fn bootstrap(actor: MemberHash, details: String) -> Self {
        Self::new(actor, ActionType::Bootstrap, details)
    }

    /// Format timestamp as human-readable relative time.
    pub fn timestamp_iso(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before UNIX epoch")
            .as_secs();

        if now < self.timestamp {
            // Future timestamp (clock skew?)
            return format!("Unix: {}", self.timestamp);
        }

        let elapsed = now - self.timestamp;

        // Format as relative time for recent entries, absolute for old ones
        match elapsed {
            0..=60 => "Just now".to_string(),
            61..=3600 => format!("{} min ago", elapsed / 60),
            3601..=86400 => format!("{} hours ago", elapsed / 3600),
            86401..=604800 => format!("{} days ago", elapsed / 86400),
            _ => format!("Unix: {}", self.timestamp),
        }
    }

    /// Format action type for display.
    pub fn action_type_display(&self) -> String {
        match &self.action_type {
            ActionType::ConfigChange => "Config Change".to_string(),
            ActionType::Restart => "Restart".to_string(),
            ActionType::ManualIntervention => "Manual Intervention".to_string(),
            ActionType::Bootstrap => "Bootstrap".to_string(),
            ActionType::Other(s) => s.clone(),
        }
    }
}

/// Query options for audit log.
#[derive(Debug, Clone)]
pub struct AuditQuery {
    /// Filter by action type.
    pub action_type: Option<ActionType>,
    /// Filter by actor.
    pub actor: Option<MemberHash>,
    /// Limit number of results (most recent first).
    pub limit: Option<usize>,
    /// Only show entries after this timestamp.
    pub after_timestamp: Option<u64>,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            action_type: None,
            actor: None,
            limit: Some(50), // Default: last 50 entries
            after_timestamp: None,
        }
    }
}

/// Query audit log with filters.
///
/// Returns entries in reverse chronological order (most recent first).
pub fn query_audit_log(entries: &[AuditEntry], query: &AuditQuery) -> Vec<AuditEntry> {
    let mut filtered: Vec<AuditEntry> = entries
        .iter()
        .filter(|entry| {
            // Filter by action type
            if let Some(ref action_type) = query.action_type {
                if &entry.action_type != action_type {
                    return false;
                }
            }

            // Filter by actor
            if let Some(ref actor) = query.actor {
                if &entry.actor != actor {
                    return false;
                }
            }

            // Filter by timestamp
            if let Some(after_ts) = query.after_timestamp {
                if entry.timestamp <= after_ts {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    // Sort by timestamp (most recent first)
    filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit
    if let Some(limit) = query.limit {
        filtered.truncate(limit);
    }

    filtered
}

/// Format audit log for display in Signal PM.
pub fn format_audit_log(entries: &[AuditEntry]) -> String {
    if entries.is_empty() {
        return "No audit entries found.".to_string();
    }

    let mut output = String::from("📋 Operator Audit Trail\n\n");

    for entry in entries {
        let actor_hash_short = hex::encode(&entry.actor.as_bytes()[..4]);
        output.push_str(&format!(
            "• {} — {} ({}…)\n  {}\n\n",
            entry.timestamp_iso(),
            entry.action_type_display(),
            actor_hash_short,
            entry.details
        ));
    }

    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_member_hash(byte: u8) -> MemberHash {
        MemberHash::from_bytes(&[byte; 32])
    }

    #[test]
    fn test_audit_entry_creation() {
        let actor = mock_member_hash(1);
        let entry = AuditEntry::config_change(actor, "Updated min_vouches from 2 to 3".to_string());

        assert_eq!(entry.actor, actor);
        assert_eq!(entry.action_type, ActionType::ConfigChange);
        assert_eq!(entry.details, "Updated min_vouches from 2 to 3");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_audit_query_filter_by_action_type() {
        let actor = mock_member_hash(1);
        let entries = vec![
            AuditEntry::config_change(actor, "Change 1".to_string()),
            AuditEntry::restart(actor, "Restart 1".to_string()),
            AuditEntry::config_change(actor, "Change 2".to_string()),
        ];

        let query = AuditQuery {
            action_type: Some(ActionType::ConfigChange),
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        assert_eq!(result.len(), 2);
        assert!(result
            .iter()
            .all(|e| e.action_type == ActionType::ConfigChange));
    }

    #[test]
    fn test_audit_query_filter_by_actor() {
        let actor1 = mock_member_hash(1);
        let actor2 = mock_member_hash(2);
        let entries = vec![
            AuditEntry::config_change(actor1, "Change 1".to_string()),
            AuditEntry::restart(actor2, "Restart 1".to_string()),
            AuditEntry::config_change(actor1, "Change 2".to_string()),
        ];

        let query = AuditQuery {
            actor: Some(actor1),
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.actor == actor1));
    }

    #[test]
    fn test_audit_query_limit() {
        let actor = mock_member_hash(1);
        let entries = vec![
            AuditEntry::config_change(actor, "Change 1".to_string()),
            AuditEntry::restart(actor, "Restart 1".to_string()),
            AuditEntry::config_change(actor, "Change 2".to_string()),
        ];

        let query = AuditQuery {
            limit: Some(2),
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_format_audit_log() {
        let actor = mock_member_hash(1);
        let entries = vec![
            AuditEntry::config_change(actor, "Updated min_vouches from 2 to 3".to_string()),
            AuditEntry::restart(actor, "Bot restarted for maintenance".to_string()),
        ];

        let formatted = format_audit_log(&entries);
        assert!(formatted.contains("Operator Audit Trail"));
        assert!(formatted.contains("Config Change"));
        assert!(formatted.contains("Restart"));
        assert!(formatted.contains("Updated min_vouches from 2 to 3"));
        assert!(formatted.contains("Bot restarted for maintenance"));
    }

    #[test]
    fn test_format_audit_log_empty() {
        let entries: Vec<AuditEntry> = vec![];
        let formatted = format_audit_log(&entries);
        assert_eq!(formatted, "No audit entries found.");
    }

    #[test]
    fn test_manual_intervention_entry() {
        let actor = mock_member_hash(1);
        let entry =
            AuditEntry::manual_intervention(actor, "Emergency ejection override".to_string());

        assert_eq!(entry.actor, actor);
        assert_eq!(entry.action_type, ActionType::ManualIntervention);
        assert_eq!(entry.details, "Emergency ejection override");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_bootstrap_entry() {
        let actor = mock_member_hash(1);
        let entry = AuditEntry::bootstrap(actor, "Created trust network group".to_string());

        assert_eq!(entry.actor, actor);
        assert_eq!(entry.action_type, ActionType::Bootstrap);
        assert_eq!(entry.details, "Created trust network group");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_action_type_other_variant() {
        let actor = mock_member_hash(1);
        let entry = AuditEntry::new(
            actor,
            ActionType::Other("CustomAction".to_string()),
            "Custom operation performed".to_string(),
        );

        assert_eq!(
            entry.action_type,
            ActionType::Other("CustomAction".to_string())
        );
        assert_eq!(entry.action_type_display(), "CustomAction");
    }

    #[test]
    fn test_timestamp_iso_future_timestamp() {
        let actor = mock_member_hash(1);
        let mut entry = AuditEntry::config_change(actor, "Test".to_string());

        // Set timestamp to future (current time + 3600 seconds)
        let future_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        entry.timestamp = future_time;

        let formatted = entry.timestamp_iso();
        // Future timestamps should return Unix format
        assert!(formatted.starts_with("Unix:"));
        assert!(formatted.contains(&future_time.to_string()));
    }

    #[test]
    fn test_timestamp_iso_relative_times() {
        let actor = mock_member_hash(1);

        // Test "Just now" (0-60 seconds)
        let mut entry = AuditEntry::config_change(actor, "Test".to_string());
        let formatted = entry.timestamp_iso();
        assert_eq!(formatted, "Just now");

        // Test minutes ago (61-3600 seconds)
        entry.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 120; // 2 minutes ago
        let formatted = entry.timestamp_iso();
        assert!(formatted.contains("min ago"));

        // Test hours ago (3601-86400 seconds)
        entry.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 7200; // 2 hours ago
        let formatted = entry.timestamp_iso();
        assert!(formatted.contains("hours ago"));

        // Test days ago (86401-604800 seconds)
        entry.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 172800; // 2 days ago
        let formatted = entry.timestamp_iso();
        assert!(formatted.contains("days ago"));

        // Test very old (>604800 seconds / 1 week)
        entry.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 1000000; // Much older
        let formatted = entry.timestamp_iso();
        assert!(formatted.starts_with("Unix:"));
    }

    #[test]
    fn test_audit_query_after_timestamp() {
        let actor = mock_member_hash(1);

        // Create entries with different timestamps
        let mut entry1 = AuditEntry::config_change(actor, "Old change".to_string());
        let mut entry2 = AuditEntry::restart(actor, "Recent restart".to_string());
        let mut entry3 = AuditEntry::config_change(actor, "New change".to_string());

        let base_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry1.timestamp = base_time - 100;
        entry2.timestamp = base_time - 50;
        entry3.timestamp = base_time - 10;

        let entries = vec![entry1, entry2, entry3];

        // Query for entries after base_time - 60
        let query = AuditQuery {
            after_timestamp: Some(base_time - 60),
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        // Should only get entry2 and entry3 (timestamps > base_time - 60)
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.timestamp > base_time - 60));
    }

    #[test]
    fn test_audit_query_combined_filters() {
        let actor1 = mock_member_hash(1);
        let actor2 = mock_member_hash(2);

        let entries = vec![
            AuditEntry::config_change(actor1, "Change 1".to_string()),
            AuditEntry::config_change(actor2, "Change 2".to_string()),
            AuditEntry::restart(actor1, "Restart 1".to_string()),
            AuditEntry::restart(actor2, "Restart 2".to_string()),
        ];

        // Filter by actor AND action type
        let query = AuditQuery {
            actor: Some(actor1),
            action_type: Some(ActionType::ConfigChange),
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].actor, actor1);
        assert_eq!(result[0].action_type, ActionType::ConfigChange);
    }

    #[test]
    fn test_audit_query_no_limit() {
        let actor = mock_member_hash(1);
        let entries: Vec<AuditEntry> = (0..100)
            .map(|i| AuditEntry::config_change(actor, format!("Change {}", i)))
            .collect();

        let query = AuditQuery {
            limit: None, // No limit
            ..Default::default()
        };

        let result = query_audit_log(&entries, &query);
        assert_eq!(result.len(), 100); // All entries returned
    }
}
