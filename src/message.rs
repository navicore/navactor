use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::oneshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct Observations {
    pub datetime: String,
    pub values: HashMap<i32, f64>,
    pub path: String,
}

/// all actor messages are delivered in envelops that contain optional
/// sender objects - these are set when a `tell` message is sent so that
/// the reply can be delivered.  These replies are not placed in envelopes.
#[derive(Debug)]
pub struct MessageEnvelope {
    pub message: Message,
    pub respond_to_opt: Option<oneshot::Sender<Message>>,
    pub timestamp: DateTime<Utc>,
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message {
    ReadAllCmd {},
    PrintOneCmd {
        text: String,
    },
    UpdateCmd {
        timestamp: DateTime<Utc>,
        path: String,
        values: HashMap<i32, f64>,
    },
    IsCompleteMsg {},
}

impl Default for MessageEnvelope {
    fn default() -> Self {
        MessageEnvelope {
            message: Message::ReadAllCmd {},
            respond_to_opt: None,
            timestamp: Utc::now(),
        }
    }
}
