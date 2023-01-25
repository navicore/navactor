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
    /// 'Query' is usually the 'ask' payload.  
    Query {
        path: String,
    },
    /// 'Update' is usually the 'tell' payload
    Update {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, f64>,
    },
    /// the response to most Query/ask interactions
    StateReport {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, f64>,
    },
    JsonParseError {
        text: String,
        datetime: OffsetDateTime,
        path: Option<String>,
    },
    JrnlError {
        text: String,
        datetime: OffsetDateTime,
        path: Option<String>,
    },
    /// the actor init process is complicated in that the actors must recalculate
    /// their state from event source replays when they are first instantiated.
    /// EndOfStream is used to complete the jrnl stream at init time.
    EndOfStream {},
    /// InitCmd instructs the actor to flip into init mode and recalculate its
    /// state from the incoming eventstream using a tokio receiver in the
    /// envelope delivering the InitCmd.
    InitCmd {},
    /// LoadCmd is sent to the persistence actor that will use the path to
    /// read and send all the actor's previous events to the new instantiation of
    /// that actor.  the tokio "sender" is presented to the actor looking up the
    /// events in the envelope delivering the LoadCmd.
    LoadCmd {
        path: String,
    },
    /// ReadAllCmd and PrintOneCmd orchestrate reads from stdin and writes to
    /// stdout in cli use cases
    ReadAllCmd {},
    PrintOneCmd {
        text: String,
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
                next_message: self.first_message,
                next_message_respond_to: self.first_message_respond_to,
                respond_to: None,
                stream_from: self.load_from,
                stream_to: None,
                message: Message::InitCmd {},
            },
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                next_message: None,
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
