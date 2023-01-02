use std::collections::HashMap;
use tokio::sync::oneshot;

/// all actor messages are delivered in envelops that contain optional
/// sender objects - these are set when a `tell` message is sent so that
/// the reply can be delivered.  These replies are not placed in envelopes.
#[derive(Debug)]
pub struct MessageEnvelope {
    pub message: Message,
    pub respond_to_opt: Option<oneshot::Sender<Message>>,
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message {
    DefineCmd { spec: String },
    ReadAllCmd {},
    PrintOneCmd { text: String },
    UpdateCmd { values: HashMap<i32, f64> },
    IsCompleteMsg {},
}
