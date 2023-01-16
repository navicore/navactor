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
    pub path: String,
    pub message: Message,
    pub respond_to_opt: Option<oneshot::Sender<Message>>,
    pub datetime: DateTime<Utc>,
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message {
    ReadAllCmd {},
    PrintOneCmd {
        text: String,
    },
    InspectCmd {},
    UpdateCmd {
        datetime: DateTime<Utc>,
        values: HashMap<i32, f64>,
    },
    StateReport {
        datetime: DateTime<Utc>,
        path: String,
        values: HashMap<i32, f64>,
    },
    ErrorReport {
        text: String,
        datetime: DateTime<Utc>,
        path: Option<String>,
    },

    IsCompleteMsg {},
}
impl Default for MessageEnvelope {
    fn default() -> Self {
        MessageEnvelope {
            path: String::from("/"),
            message: Message::ReadAllCmd {},
            respond_to_opt: None,
            datetime: Utc::now(),
        }
    }
}
