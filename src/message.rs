//! Message types for the distributed queue wire protocol.
//!
//! Defines the internal `Message` representation and the client/server
//! protocol enums that are serialized as tagged JSON over TCP.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Internal message stored in the broker's queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub payload: String,
    /// Unix timestamp (seconds since epoch) when the message was created.
    pub timestamp: u64,
}

/// Messages sent from clients (producers/consumers) to the broker.
///
/// Uses `#[serde(tag = "type")]` for internally-tagged JSON — each variant
/// serializes with a `"type"` field (e.g. `{"type": "publish", "payload": "..."}`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// First message a client sends to identify itself as producer or consumer.
    #[serde(rename = "register")]
    Register { role: String },
    /// Producer sends this to enqueue a new message.
    #[serde(rename = "publish")]
    Publish { payload: String },
    /// Consumer sends this to acknowledge successful processing of a message.
    #[serde(rename = "ack")]
    Ack { id: u64 },
}

/// Messages sent from the broker to clients.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Delivers a queued message to a consumer.
    #[serde(rename = "message")]
    Message { id: u64, payload: String },
    /// Generic success response (e.g. after a publish).
    #[serde(rename = "ok")]
    Ok,
    /// Error response with a human-readable description.
    #[serde(rename = "error")]
    Error { message: String },
}

impl Message {
    /// Creates a new message with the given id and payload, timestamped to now.
    pub fn new(id: u64, payload: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Message { id, payload, timestamp }
    }
}