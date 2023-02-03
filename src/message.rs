use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type ActorResult<T> = std::result::Result<T, ActorError>;

#[derive(Debug, Clone)]
pub struct ActorError {
    pub reason: String,
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bad actor state: {}", self.reason)
    }
}

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
    pub respond_to: Option<oneshot::Sender<ActorResult<Message>>>,
    pub datetime: OffsetDateTime,
    pub stream_to: Option<mpsc::Sender<Message>>,
    pub stream_from: Option<mpsc::Receiver<Message>>,
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
        Self {
            message: Message::ReadAllCmd {},
            respond_to: None,
            datetime: OffsetDateTime::now_utc(),
            stream_to: None,
            stream_from: None,
        }
    }
}

struct LifeCycleBuilder {
    load_from: Option<mpsc::Receiver<Message>>,
    send_to: Option<mpsc::Sender<Message>>,
    send_to_path: Option<String>,
    respond_to: Option<oneshot::Sender<ActorResult<Message>>>,
}

impl LifeCycleBuilder {
    fn new() -> Self {
        Self {
            load_from: None,
            send_to: None,
            send_to_path: None,
            respond_to: None,
        }
    }

    fn with_respond_to(mut self, respond_to: oneshot::Sender<ActorResult<Message>>) -> Self {
        self.respond_to = Some(respond_to);
        self
    }

    fn with_load_from(mut self, load_from: mpsc::Receiver<Message>) -> Self {
        self.load_from = Some(load_from);
        self
    }

    fn with_send_to(mut self, send_to: mpsc::Sender<Message>, send_to_path: String) -> Self {
        self.send_to = Some(send_to);
        self.send_to_path = Some(send_to_path);
        self
    }

    fn build(self) -> (MessageEnvelope, MessageEnvelope) {
        (
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                respond_to: self.respond_to,
                stream_from: self.load_from,
                stream_to: None,
                message: Message::InitCmd {},
            },
            MessageEnvelope {
                datetime: OffsetDateTime::now_utc(),
                respond_to: None,
                stream_from: None,
                stream_to: self.send_to,
                message: Message::LoadCmd {
                    path: self.send_to_path.unwrap_or_default(),
                },
            },
        )
    }
}

// factory function
#[must_use]
pub fn create_init_lifecycle(
    path: String,
    bufsz: usize,
    respond_to: oneshot::Sender<ActorResult<Message>>,
) -> (MessageEnvelope, MessageEnvelope) {
    let (tx, rx) = mpsc::channel(bufsz);
    let builder = LifeCycleBuilder::new()
        .with_load_from(rx)
        .with_send_to(tx, path)
        .with_respond_to(respond_to);
    builder.build()
}
