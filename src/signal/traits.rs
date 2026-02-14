//! Signal Client Trait Abstractions
//!
//! These traits enable 100% test coverage via MockSignalClient.
//! See: .beads/security-constraints.bead § 10 (Code Quality for Security)

use async_trait::async_trait;
use std::fmt;

/// Signal service identifier (ACI or PNI)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId(pub String);

/// Signal group identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GroupId(pub Vec<u8>);

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

/// Group information (metadata and settings)
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub name: String,
    pub description: Option<String>,
    pub disappearing_messages_timer: Option<u32>, // seconds, 0 = off
    pub announcements_only: bool,
}

/// Message source (DM or group context)
///
/// Per Stroma architecture, bot belongs to ONE Signal group only.
/// When message is from group context, bot responds to its configured group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSource {
    /// Direct message (1-on-1 PM)
    DirectMessage,
    /// Message from a group context (bot will respond to its configured group)
    Group,
}

/// Signal message content
#[derive(Debug, Clone)]
pub struct Message {
    pub sender: ServiceId,
    pub source: MessageSource,
    pub content: MessageContent,
    pub timestamp: u64,
}

/// Message content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Poll(Poll),
    PollVote(PollVote),
}

/// Poll structure (Signal Protocol v8)
#[derive(Debug, Clone)]
pub struct Poll {
    pub question: String,
    pub options: Vec<String>,
}

/// Poll vote
#[derive(Debug, Clone)]
pub struct PollVote {
    pub poll_id: u64,
    pub selected_options: Vec<u32>,
}

/// Result type for Signal operations
pub type SignalResult<T> = Result<T, SignalError>;

/// Signal client errors
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Member not found: {0}")]
    MemberNotFound(String),

    #[error("Unauthorized operation")]
    Unauthorized,

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Signal Client abstraction for testing
///
/// This trait enables 100% test coverage by allowing MockSignalClient
/// to replace real Presage Manager in tests.
///
/// Uses `#[async_trait(?Send)]` because presage Manager produces !Send futures.
/// All implementations must run on a tokio::task::LocalSet.
#[async_trait(?Send)]
pub trait SignalClient: Clone {
    /// Send text message to a recipient
    async fn send_message(&self, recipient: &ServiceId, text: &str) -> SignalResult<()>;

    /// Send message to a group
    async fn send_group_message(&self, group: &GroupId, text: &str) -> SignalResult<()>;

    /// Create a new group with initial members
    ///
    /// Members are specified as ServiceIds. The implementation looks up profile keys
    /// from the presage store. Members without profile keys are added as pending
    /// invites (matching Signal Desktop behavior).
    ///
    /// Returns the GroupId and a list of pending member ServiceIds that need
    /// group invite DMs sent separately (via `send_group_invite`).
    async fn create_group(
        &self,
        name: &str,
        members: &[ServiceId],
    ) -> SignalResult<(GroupId, Vec<ServiceId>)>;

    /// Send a group invite DM to a pending member.
    ///
    /// This sends a DataMessage with GroupContextV2 so the recipient's Signal
    /// client displays the group invite. Should be called after `create_group`
    /// returns, when the websocket connection is healthy.
    async fn send_group_invite(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()>;

    /// Add member to group
    async fn add_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()>;

    /// Remove member from group
    async fn remove_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()>;

    /// Create poll in group with up to 10 options
    ///
    /// # Arguments
    /// * `group` - Group to create poll in
    /// * `question` - Poll question
    /// * `options` - Answer options (up to 10)
    /// * `allow_multiple` - Whether voters can select multiple options
    ///
    /// # Returns
    /// * Poll timestamp (needed for voting/terminating)
    async fn create_poll(
        &self,
        group: &GroupId,
        question: &str,
        options: Vec<String>,
        allow_multiple: bool,
    ) -> SignalResult<u64>;

    /// Terminate a poll (closes voting)
    ///
    /// Per proposal-system.bead:
    /// - Sends PollTerminate message to group
    /// - Prevents late votes after timeout
    /// - Visual feedback in Signal UI (shows as closed)
    async fn terminate_poll(&self, group: &GroupId, poll_timestamp: u64) -> SignalResult<()>;

    /// Get group information (name, description, settings)
    async fn get_group_info(&self, group: &GroupId) -> SignalResult<GroupInfo>;

    /// Set group name (1-32 characters)
    async fn set_group_name(&self, group: &GroupId, name: &str) -> SignalResult<()>;

    /// Set group description (0-480 characters, empty string to clear)
    async fn set_group_description(&self, group: &GroupId, description: &str) -> SignalResult<()>;

    /// Set disappearing messages timer (0 = off, otherwise seconds)
    async fn set_disappearing_messages(&self, group: &GroupId, seconds: u32) -> SignalResult<()>;

    /// Set announcements-only mode (true = only admins can send messages)
    async fn set_announcements_only(&self, group: &GroupId, enabled: bool) -> SignalResult<()>;

    /// Resolve a user identifier (UUID, username, or phone number) to a ServiceId.
    ///
    /// Accepts:
    /// - Raw UUIDs: "a1b2c3d4-5678-90ab-cdef-1234567890ab"
    /// - Usernames: "matt.42" or "@matt.42"
    /// - Phone numbers: "+15551234567" (Phase 2, not yet implemented)
    ///
    /// Returns error if the identifier cannot be resolved or is invalid.
    async fn resolve_identifier(&self, identifier: &str) -> SignalResult<ServiceId>;

    /// Receive messages (blocking until message arrives)
    async fn receive_messages(&self) -> SignalResult<Vec<Message>>;

    /// Get bot's own service ID
    fn service_id(&self) -> &ServiceId;
}
