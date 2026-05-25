use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "register")]
    Register { role: String },
    #[serde(rename = "publish")]
    Publish { payload: String },
    #[serde(rename = "ack")]
    Ack { id: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "message")]
    Message { id: u64, payload: String },
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error { message: String },
}

impl Message {
    pub fn new(id: u64, payload: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Message { id, payload, timestamp }
    }
}