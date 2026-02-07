//! Ephemeral Vetting Session Management
//!
//! CRITICAL: All vetting context is EPHEMERAL (never persisted to disk or Freenet).
//! Sessions are in-memory only and deleted after admission/rejection.
//!
//! See: hq-n9v § Context is EPHEMERAL

use crate::freenet::contract::MemberHash;
use crate::signal::traits::ServiceId;
use std::collections::HashMap;

/// Ephemeral vetting session data
///
/// CRITICAL: This data is NEVER persisted to disk or Freenet contract.
/// It exists only in memory during the vetting process and is deleted
/// after admission or rejection.
#[derive(Debug, Clone)]
pub struct VettingSession {
    /// Invitee's ServiceId (Signal identity)
    pub invitee_id: ServiceId,

    /// Invitee's username/display name
    pub invitee_username: String,

    /// Inviter's member hash (first voucher)
    pub inviter: MemberHash,

    /// Inviter's ServiceId (for notification)
    pub inviter_id: ServiceId,

    /// Context about the invitee (EPHEMERAL - never persisted)
    pub context: Option<String>,

    /// Selected validator's member hash (Blind Matchmaker selection)
    pub validator: Option<MemberHash>,

    /// Selected validator's ServiceId (for PM)
    pub validator_id: Option<ServiceId>,

    /// Current status of vetting
    pub status: VettingStatus,

    /// Whether invitee has previous flags (GAP-10)
    pub has_previous_flags: bool,

    /// Count of previous flags if any (GAP-10)
    pub previous_flag_count: u32,
}

/// Vetting session status
#[derive(Debug, Clone, PartialEq)]
pub enum VettingStatus {
    /// Pending validator selection
    PendingMatch,

    /// PMs sent, waiting for validator vouch
    AwaitingVouch,

    /// Completed - admitted to group
    Admitted,

    /// Rejected - did not receive second vouch
    Rejected,
}

/// In-memory vetting session manager
///
/// CRITICAL: All sessions are ephemeral (RAM only, no persistence).
/// Sessions are deleted immediately after admission/rejection.
pub struct VettingSessionManager {
    /// Active sessions: invitee_username -> session
    /// Uses username as key for command lookup (/vouch @username)
    sessions: HashMap<String, VettingSession>,
}

impl VettingSessionManager {
    /// Create new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create new vetting session
    #[allow(clippy::too_many_arguments)]
    pub fn create_session(
        &mut self,
        invitee_id: ServiceId,
        invitee_username: String,
        inviter: MemberHash,
        inviter_id: ServiceId,
        context: Option<String>,
        has_previous_flags: bool,
        previous_flag_count: u32,
    ) -> Result<(), VettingError> {
        // Check if session already exists
        if self.sessions.contains_key(&invitee_username) {
            return Err(VettingError::SessionAlreadyExists(invitee_username.clone()));
        }

        let session = VettingSession {
            invitee_id,
            invitee_username: invitee_username.clone(),
            inviter,
            inviter_id,
            context,
            validator: None,
            validator_id: None,
            status: VettingStatus::PendingMatch,
            has_previous_flags,
            previous_flag_count,
        };

        self.sessions.insert(invitee_username, session);
        Ok(())
    }

    /// Get active session for invitee
    pub fn get_session(&self, invitee_username: &str) -> Option<&VettingSession> {
        self.sessions.get(invitee_username)
    }

    /// Get mutable session for invitee
    pub fn get_session_mut(&mut self, invitee_username: &str) -> Option<&mut VettingSession> {
        self.sessions.get_mut(invitee_username)
    }

    /// Assign validator to session (Blind Matchmaker result)
    pub fn assign_validator(
        &mut self,
        invitee_username: &str,
        validator: MemberHash,
        validator_id: ServiceId,
    ) -> Result<(), VettingError> {
        let session = self
            .sessions
            .get_mut(invitee_username)
            .ok_or_else(|| VettingError::SessionNotFound(invitee_username.to_string()))?;

        session.validator = Some(validator);
        session.validator_id = Some(validator_id);
        session.status = VettingStatus::AwaitingVouch;

        Ok(())
    }

    /// Mark session as admitted and delete it (ephemeral)
    pub fn admit(&mut self, invitee_username: &str) -> Result<VettingSession, VettingError> {
        let mut session = self
            .sessions
            .remove(invitee_username)
            .ok_or_else(|| VettingError::SessionNotFound(invitee_username.to_string()))?;

        session.status = VettingStatus::Admitted;
        Ok(session)
    }

    /// Mark session as rejected and delete it (ephemeral)
    pub fn reject(&mut self, invitee_username: &str) -> Result<VettingSession, VettingError> {
        let mut session = self
            .sessions
            .remove(invitee_username)
            .ok_or_else(|| VettingError::SessionNotFound(invitee_username.to_string()))?;

        session.status = VettingStatus::Rejected;
        Ok(session)
    }

    /// Clear all sessions (e.g., on bot restart - sessions are ephemeral)
    pub fn clear_all(&mut self) {
        self.sessions.clear();
    }

    /// Get count of active sessions
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for VettingSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Vetting session errors
#[derive(Debug, thiserror::Error)]
pub enum VettingError {
    #[error("Vetting session already exists for {0}")]
    SessionAlreadyExists(String),

    #[error("Vetting session not found for {0}")]
    SessionNotFound(String),
}

// ============================================================================
// Privacy-Safe Message Templates
// ============================================================================
//
// CRITICAL PRIVACY CONSTRAINTS:
// 1. NEVER reveal inviter identity to assessor
// 2. NEVER reveal assessor identity to inviter
// 3. NO Signal IDs in cleartext messages
//
// These functions generate notification messages that respect privacy boundaries
// during the vetting process.

/// Message to assessor: Assessment request
///
/// PRIVACY: Contains context and invitee contact, but NO inviter identity.
/// The assessor should not know who made the original invitation.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee (e.g., "@alice")
/// * `context` - Optional context about the invitee
/// * `has_previous_flags` - Whether invitee has previous flags (GAP-10)
/// * `previous_flag_count` - Count of previous flags if any
pub fn msg_assessment_request(
    invitee_username: &str,
    context: Option<&str>,
    has_previous_flags: bool,
    previous_flag_count: u32,
) -> String {
    let mut msg = format!(
        "🔍 Assessment Request\n\n\
         You've been selected to assess a candidate: {}\n\n",
        invitee_username
    );

    if let Some(ctx) = context {
        msg.push_str(&format!("Context: {}\n\n", ctx));
    }

    if has_previous_flags {
        msg.push_str(&format!(
            "⚠️ Note: This candidate has {} previous flag(s).\n\n",
            previous_flag_count
        ));
    }

    msg.push_str(&format!(
        "If you wish to vouch for them, use:\n\
         /vouch {}\n\n\
         If you prefer not to vouch, no action is needed.",
        invitee_username
    ));

    msg
}

/// Message to inviter: Confirmation that assessment process started
///
/// PRIVACY: NO assessor identity revealed to the inviter.
/// The inviter should not know who is assessing their invite.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee
pub fn msg_inviter_confirmation(invitee_username: &str) -> String {
    format!(
        "✓ Vetting Process Started\n\n\
         Your invitation for {} has been received.\n\n\
         A validator has been selected and will assess the candidate.\n\
         You'll be notified when the process completes.",
        invitee_username
    )
}

/// Message to assessor: Rejection acknowledgment
///
/// PRIVACY: Brief acknowledgment only, no identity disclosure.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee
pub fn msg_rejection_ack(invitee_username: &str) -> String {
    format!(
        "📋 Assessment Complete\n\n\
         The vetting process for {} has concluded.\n\
         Thank you for your participation.",
        invitee_username
    )
}

/// Message to assessor: Vouch recorded, showing threshold progress
///
/// PRIVACY: Shows progress toward admission threshold, no identity disclosure.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee
/// * `current_vouches` - Current number of vouches
/// * `required_threshold` - Required threshold for admission
pub fn msg_vouch_recorded(
    invitee_username: &str,
    current_vouches: u32,
    required_threshold: u32,
) -> String {
    format!(
        "✓ Vouch Recorded\n\n\
         Your vouch for {} has been recorded.\n\n\
         Progress: {}/{} vouches toward admission threshold.",
        invitee_username, current_vouches, required_threshold
    )
}

/// Message to inviter: No candidates found (stalled state)
///
/// PRIVACY: Generic stall notification, no specific validator information.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee
pub fn msg_no_candidates(invitee_username: &str) -> String {
    format!(
        "⏸️ Vetting Paused\n\n\
         The vetting process for {} is temporarily paused.\n\n\
         No suitable validators are currently available.\n\
         The process will resume automatically when validators become available.",
        invitee_username
    )
}

/// Message to inviter: Admission successful
///
/// PRIVACY: Success notification only, no assessor identity revealed.
///
/// # Arguments
/// * `invitee_username` - Username of the invitee
pub fn msg_admission_success(invitee_username: &str) -> String {
    format!(
        "🎉 Admission Complete\n\n\
         {} has been admitted to the group!\n\n\
         The vetting process completed successfully and they now have full membership.",
        invitee_username
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member_hash(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    fn test_service_id(name: &str) -> ServiceId {
        ServiceId(name.to_string())
    }

    #[test]
    fn test_create_session() {
        let mut manager = VettingSessionManager::new();

        let result = manager.create_session(
            test_service_id("alice_id"),
            "@alice".to_string(),
            test_member_hash(1),
            test_service_id("bob_id"),
            Some("Great activist".to_string()),
            false,
            0,
        );

        assert!(result.is_ok());
        assert_eq!(manager.active_count(), 1);

        let session = manager.get_session("@alice").unwrap();
        assert_eq!(session.invitee_username, "@alice");
        assert_eq!(session.context, Some("Great activist".to_string()));
        assert_eq!(session.status, VettingStatus::PendingMatch);
        assert!(!session.has_previous_flags);
    }

    #[test]
    fn test_duplicate_session_fails() {
        let mut manager = VettingSessionManager::new();

        manager
            .create_session(
                test_service_id("alice_id"),
                "@alice".to_string(),
                test_member_hash(1),
                test_service_id("bob_id"),
                None,
                false,
                0,
            )
            .unwrap();

        let result = manager.create_session(
            test_service_id("alice_id2"),
            "@alice".to_string(),
            test_member_hash(2),
            test_service_id("carol_id"),
            None,
            false,
            0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_assign_validator() {
        let mut manager = VettingSessionManager::new();

        manager
            .create_session(
                test_service_id("alice_id"),
                "@alice".to_string(),
                test_member_hash(1),
                test_service_id("bob_id"),
                None,
                false,
                0,
            )
            .unwrap();

        let result =
            manager.assign_validator("@alice", test_member_hash(3), test_service_id("carol_id"));

        assert!(result.is_ok());

        let session = manager.get_session("@alice").unwrap();
        assert_eq!(session.validator, Some(test_member_hash(3)));
        assert_eq!(session.status, VettingStatus::AwaitingVouch);
    }

    #[test]
    fn test_admit_deletes_session() {
        let mut manager = VettingSessionManager::new();

        manager
            .create_session(
                test_service_id("alice_id"),
                "@alice".to_string(),
                test_member_hash(1),
                test_service_id("bob_id"),
                None,
                false,
                0,
            )
            .unwrap();

        let result = manager.admit("@alice");
        assert!(result.is_ok());

        let admitted_session = result.unwrap();
        assert_eq!(admitted_session.status, VettingStatus::Admitted);

        // Session should be deleted
        assert_eq!(manager.active_count(), 0);
        assert!(manager.get_session("@alice").is_none());
    }

    #[test]
    fn test_session_with_previous_flags() {
        let mut manager = VettingSessionManager::new();

        manager
            .create_session(
                test_service_id("alice_id"),
                "@alice".to_string(),
                test_member_hash(1),
                test_service_id("bob_id"),
                None,
                true,
                3,
            )
            .unwrap();

        let session = manager.get_session("@alice").unwrap();
        assert!(session.has_previous_flags);
        assert_eq!(session.previous_flag_count, 3);
    }
}
