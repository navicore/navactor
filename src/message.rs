use chrono::DateTime;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::oneshot;

/// all actor messages are delivered in envelops that contain optional
/// sender objects - these are set when a `tell` message is sent so that
/// the reply can be delivered.  These replies are not placed in envelopes.
#[derive(Debug)]
pub struct MessageEnvelope<'a> {
    pub message: Message<'a>,
    pub respond_to_opt: Option<oneshot::Sender<Message<'a>>>,
    pub timestamp: DateTime<Utc>,
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message<'a> {
    ReadAllCmd {},
    PrintOneCmd {
        text: String,
    },
    UpdateCmd {
        path: &'a Path,
        values: HashMap<i32, f64>,
    },
    IsCompleteMsg {},
}

impl<'a> Default for MessageEnvelope<'a> {
    fn default() -> Self {
        MessageEnvelope {
            message: Message::ReadAllCmd {},
            respond_to_opt: None,
            timestamp: Utc::now(),
        }
    }
}
