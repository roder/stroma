//! Signal Client Trait Abstractions
//!
//! These traits enable 100% test coverage via MockSignalClient.
//! See: .beads/security-constraints.bead § 10 (Code Quality for Security)

use async_trait::async_trait;
use std::fmt;

/// Signal service identifier (ACI or PNI)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId(pub String);

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Signal group identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupId(pub Vec<u8>);

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

/// Signal message content
#[derive(Debug, Clone)]
pub struct Message {
    pub sender: ServiceId,
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
#[async_trait]
pub trait SignalClient: Send + Sync + Clone {
    /// Send text message to a recipient
    async fn send_message(&self, recipient: &ServiceId, text: &str) -> SignalResult<()>;

    /// Send message to a group
    async fn send_group_message(&self, group: &GroupId, text: &str) -> SignalResult<()>;

    /// Add member to group
    async fn add_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()>;

    /// Remove member from group
    async fn remove_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()>;

    /// Create poll in group
    async fn create_poll(&self, group: &GroupId, poll: &Poll) -> SignalResult<u64>;

    /// Receive messages (blocking until message arrives)
    async fn receive_messages(&self) -> SignalResult<Vec<Message>>;

    /// Get bot's own service ID
    fn service_id(&self) -> &ServiceId;
}
