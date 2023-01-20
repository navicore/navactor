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
    pub respond_to: Option<oneshot::Sender<Message>>,
    pub datetime: OffsetDateTime,
    pub stream_to: Option<mpsc::Sender<Message>>,
    pub stream_from: Option<mpsc::Receiver<Message>>,
    pub next_message: Option<Message>,
    pub next_message_respond_to: Option<oneshot::Sender<Message>>,
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
    LoadCmd {
        path: String,
    },
}

impl Default for MessageEnvelope {
    fn default() -> Self {
        MessageEnvelope {
            message: Message::ReadAllCmd {},
            respond_to: None,
            datetime: OffsetDateTime::now_utc(),
            stream_to: None,
            stream_from: None,
            next_message: None,
            next_message_respond_to: None,
        }
    }
}

struct LifeCycleBuilder {
    load_from: Option<mpsc::Receiver<Message>>,
    send_to: Option<mpsc::Sender<Message>>,
    send_to_path: Option<String>,
    first_message: Option<Message>,
    first_message_respond_to: Option<oneshot::Sender<Message>>,
}

impl LifeCycleBuilder {
    fn new() -> LifeCycleBuilder {
        LifeCycleBuilder {
            load_from: None,
            send_to: None,
            send_to_path: None,
            first_message: None,
            first_message_respond_to: None,
        }
    }

    fn with_load_from(mut self, load_from: mpsc::Receiver<Message>) -> LifeCycleBuilder {
        self.load_from = Some(load_from);
        self
    }

    fn with_send_to(
        mut self,
        send_to: mpsc::Sender<Message>,
        send_to_path: String,
    ) -> LifeCycleBuilder {
        self.send_to = Some(send_to);
        self.send_to_path = Some(send_to_path);
        self
    }

    fn with_first_message(mut self, first_message: Message) -> LifeCycleBuilder {
        self.first_message = Some(first_message);
        self
    }

    fn with_first_message_respond_to(
        mut self,
        first_message_respond_to: Option<oneshot::Sender<Message>>,
    ) -> LifeCycleBuilder {
        self.first_message_respond_to = first_message_respond_to;
        self
    }

    fn build(self) -> (MessageEnvelope, MessageEnvelope) {
        (
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                next_message: None,
                next_message_respond_to: self.first_message_respond_to,
                respond_to: None,
                stream_from: self.load_from,
                stream_to: None,
                message: Message::InitCmd {},
            },
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                next_message: self.first_message,
                next_message_respond_to: None,
                respond_to: None,
                stream_from: None,
                stream_to: self.send_to,
                message: Message::LoadCmd {
                    path: self.send_to_path.unwrap(),
                },
            },
        )
    }
}

// factory function
pub fn create_init_lifecycle(
    message: Message,
    path: String,
    bufsz: usize,
    send: Option<oneshot::Sender<Message>>,
) -> (MessageEnvelope, MessageEnvelope) {
    let (tx, rx) = mpsc::channel(bufsz);
    let builder = LifeCycleBuilder::new()
        .with_load_from(rx)
        .with_send_to(tx, path)
        .with_first_message(message)
        .with_first_message_respond_to(send);
    builder.build()
}
