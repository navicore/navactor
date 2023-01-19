use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::mpsc;
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
    pub datetime: OffsetDateTime,
    pub stream_to: Option<mpsc::Sender<Message>>,
    pub stream_from: Option<mpsc::Receiver<Message>>,
    pub next_message: Option<Message>,
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message {
    ReadAllCmd {},
    PrintOneCmd {
        text: String,
    },
    InspectCmd {
        path: String,
    },
    UpdateCmd {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, f64>,
    },
    StateReport {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, f64>,
    },
    ErrorReport {
        text: String,
        datetime: OffsetDateTime,
        path: Option<String>,
    },

    EndOfStream {},
    InitCmd {},
    LoadCmd {},
}

impl Default for MessageEnvelope {
    fn default() -> Self {
        MessageEnvelope {
            message: Message::ReadAllCmd {},
            respond_to_opt: None,
            datetime: OffsetDateTime::now_utc(),
            stream_to: None,
            stream_from: None,
            next_message: None,
        }
    }
}

struct LifeCycleBuilder {
    load_from: Option<mpsc::Receiver<Message>>,
    send_to: Option<mpsc::Sender<Message>>,
    first_message: Option<Message>,
}

impl LifeCycleBuilder {
    fn new() -> LifeCycleBuilder {
        LifeCycleBuilder {
            load_from: None,
            send_to: None,
            first_message: None,
        }
    }

    fn with_load_from(mut self, load_from: mpsc::Receiver<Message>) -> LifeCycleBuilder {
        self.load_from = Some(load_from);
        self
    }

    fn with_send_to(mut self, send_to: mpsc::Sender<Message>) -> LifeCycleBuilder {
        self.send_to = Some(send_to);
        self
    }

    fn with_first_message(mut self, first_message: Message) -> LifeCycleBuilder {
        self.first_message = Some(first_message);
        self
    }

    fn build(self) -> (MessageEnvelope, MessageEnvelope) {
        (
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                next_message: None,
                respond_to_opt: None,
                stream_from: self.load_from,
                stream_to: None,
                message: Message::InitCmd {},
            },
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                next_message: self.first_message,
                respond_to_opt: None,
                stream_from: None,
                stream_to: self.send_to,
                message: Message::LoadCmd {},
            },
        )
    }
}

// factory function
pub fn create_init_lifecycle(
    first_message: Message,
    bufsz: usize,
) -> (MessageEnvelope, MessageEnvelope) {
    let (tx, rx) = mpsc::channel(bufsz);
    let builder = LifeCycleBuilder::new()
        .with_load_from(rx)
        .with_send_to(tx)
        .with_first_message(first_message);
    builder.build()
}
