//! Display layer for strategic introduction suggestions.
//!
//! Per blind-matchmaker-dvr.bead § "UX Messages":
//! - Bot uses **Signal display names** in user-facing messages, never hashes
//! - Bot maintains transient in-memory mapping (Signal ID → hash)
//! - Mapping is reconstructible on restart (HMAC-based)
//!
//! The bot's ACI-derived key provides namespace isolation.

use crate::freenet::contract::MemberHash;
use crate::matchmaker::strategic_intro::Introduction;
use std::collections::HashMap;

/// User-facing introduction message
#[derive(Debug, Clone)]
pub struct IntroductionMessage {
    /// The formatted message to display to the user
    pub message: String,

    /// Priority level (0 = highest, DVR-optimal)
    pub priority: u8,

    /// Whether this is a DVR-optimal suggestion
    pub dvr_optimal: bool,
}

/// Resolve member hash to Signal display name
///
/// Per blind-matchmaker-dvr.bead:
/// - Bot maintains transient mapping (Signal ID → hash) in session state
/// - If member's Signal profile isn't cached, prompt user to refresh
///
/// Note: This is a placeholder. The actual implementation would use the
/// bot's session state to look up the Signal display name.
pub fn resolve_display_name(
    member: &MemberHash,
    display_names: &HashMap<MemberHash, String>,
) -> String {
    display_names
        .get(member)
        .cloned()
        .unwrap_or_else(|| format!("@Unknown_{:02x}", member.as_bytes()[0]))
}

/// Format introduction as user-facing message
///
/// Per blind-matchmaker-dvr.bead § "UX Messages":
/// - DVR-optimal: "Strategic Introduction" with explanation of network benefits
/// - MST fallback: "Suggestion" with connectivity benefits
pub fn format_introduction(
    intro: &Introduction,
    display_names: &HashMap<MemberHash, String>,
) -> IntroductionMessage {
    let person_b_name = resolve_display_name(&intro.person_b, display_names);

    let message = if intro.dvr_optimal {
        // DVR-optimal suggestion
        format!(
            "🔗 Strategic Introduction: I've identified {} as an ideal \
            connection for you. Vouching for them would strengthen the network's \
            distributed trust (they'd become independently verified).\n\n\
            Would you like me to facilitate an introduction?",
            person_b_name
        )
    } else if intro.priority == 1 {
        // MST fallback suggestion
        format!(
            "🔗 Suggestion: Consider connecting with {} from a different \
            part of the network. This would strengthen your position and improve \
            network connectivity.",
            person_b_name
        )
    } else {
        // Cluster bridging
        format!(
            "🔗 Bridge Suggestion: Connecting with {} would help bridge \
            separate parts of the network, improving overall resilience.",
            person_b_name
        )
    };

    IntroductionMessage {
        message,
        priority: intro.priority,
        dvr_optimal: intro.dvr_optimal,
    }
}

/// Format multiple introductions as a prioritized list
///
/// Per blind-matchmaker-dvr.bead § "Bot Behavior by Health Status":
/// - Unhealthy (DVR 0-33%): Aggressively suggest DVR-optimal
/// - Developing (DVR 33-66%): Suggest DVR-optimal when convenient
/// - Healthy (DVR 66-100%): Suggest MST (maintenance mode)
pub fn format_introduction_list(
    introductions: &[Introduction],
    display_names: &HashMap<MemberHash, String>,
    max_suggestions: usize,
) -> Vec<IntroductionMessage> {
    let mut sorted_intros = introductions.to_vec();

    // Sort by priority (0 = highest)
    sorted_intros.sort_by_key(|intro| intro.priority);

    // Take top N suggestions
    sorted_intros
        .iter()
        .take(max_suggestions)
        .map(|intro| format_introduction(intro, display_names))
        .collect()
}

/// Calculate DVR (Distinct Validator Ratio) for health status
///
/// Per blind-matchmaker-dvr.bead § "Relationship to DVR Metric":
/// DVR = Distinct_Validators / floor(N / 4)
///
/// Note: This is a simplified calculation. The actual implementation would
/// need to determine which Validators are truly "distinct" (non-overlapping
/// voucher sets).
pub fn calculate_dvr(validator_count: usize, total_members: usize) -> f64 {
    if total_members < 4 {
        return 1.0; // Bootstrap case
    }

    let expected_validators = total_members / 4;
    if expected_validators == 0 {
        return 1.0;
    }

    (validator_count as f64) / (expected_validators as f64)
}

/// Get health status based on DVR
///
/// Per blind-matchmaker-dvr.bead § "Bot Behavior by Health Status":
/// - Unhealthy: 0-33%
/// - Developing: 33-66%
/// - Healthy: 66-100%
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Unhealthy,
    Developing,
    Healthy,
}

impl HealthStatus {
    pub fn from_dvr(dvr: f64) -> Self {
        if dvr < 0.33 {
            HealthStatus::Unhealthy
        } else if dvr < 0.66 {
            HealthStatus::Developing
        } else {
            HealthStatus::Healthy
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            HealthStatus::Unhealthy => "🔴",
            HealthStatus::Developing => "🟡",
            HealthStatus::Healthy => "🟢",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            HealthStatus::Unhealthy => "Unhealthy",
            HealthStatus::Developing => "Developing",
            HealthStatus::Healthy => "Healthy",
        }
    }
}

/// Format health status message
pub fn format_health_status(dvr: f64) -> String {
    let status = HealthStatus::from_dvr(dvr);
    format!(
        "{} Network Health: {} (DVR: {:.1}%)",
        status.emoji(),
        status.description(),
        dvr * 100.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::contract::MemberHash;

    fn member_hash(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash::from_bytes(&bytes)
    }

    #[test]
    fn test_resolve_display_name_known() {
        let member = member_hash(1);
        let mut names = HashMap::new();
        names.insert(member, "Alice".to_string());

        let name = resolve_display_name(&member, &names);
        assert_eq!(name, "Alice");
    }

    #[test]
    fn test_resolve_display_name_unknown() {
        let member = member_hash(1);
        let names = HashMap::new();

        let name = resolve_display_name(&member, &names);
        assert!(name.starts_with("@Unknown_"));
    }

    #[test]
    fn test_format_dvr_optimal() {
        let intro = Introduction {
            person_a: member_hash(1),
            person_b: member_hash(2),
            reason: "DVR optimization".to_string(),
            priority: 0,
            dvr_optimal: true,
        };

        let mut names = HashMap::new();
        names.insert(member_hash(2), "Bob".to_string());

        let msg = format_introduction(&intro, &names);
        assert!(msg.message.contains("Strategic Introduction"));
        assert!(msg.message.contains("Bob"));
        assert!(msg.dvr_optimal);
    }

    #[test]
    fn test_format_mst_fallback() {
        let intro = Introduction {
            person_a: member_hash(1),
            person_b: member_hash(2),
            reason: "MST optimization".to_string(),
            priority: 1,
            dvr_optimal: false,
        };

        let mut names = HashMap::new();
        names.insert(member_hash(2), "Carol".to_string());

        let msg = format_introduction(&intro, &names);
        assert!(msg.message.contains("Suggestion"));
        assert!(msg.message.contains("Carol"));
        assert!(!msg.dvr_optimal);
    }

    #[test]
    fn test_calculate_dvr() {
        // 4 validators out of 20 members = expected 5, DVR = 0.8
        let dvr = calculate_dvr(4, 20);
        assert_eq!(dvr, 0.8);

        // Bootstrap case
        let dvr_bootstrap = calculate_dvr(0, 3);
        assert_eq!(dvr_bootstrap, 1.0);
    }

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::from_dvr(0.2), HealthStatus::Unhealthy);
        assert_eq!(HealthStatus::from_dvr(0.5), HealthStatus::Developing);
        assert_eq!(HealthStatus::from_dvr(0.8), HealthStatus::Healthy);
    }

    #[test]
    fn test_format_health_status() {
        let msg = format_health_status(0.75);
        assert!(msg.contains("Healthy"));
        assert!(msg.contains("75"));
    }
}
