//! The `message` module provides types and functions that define the communication
//! protocol for actors in the system. The core of the communication protocol is a set of
//! message types (`Message<T>`) that are sent to actors and define the operations to be
//! performed on the state of an actor or to request information from the state of an actor.
//!
//! The `Envelope<T>` struct wraps `Message<T>` types, along with metadata about the sender,
//! receiver, and timing of the message. `Envelope<T>` is used to communicate with actors
//! in the system.
//!
//! The `PathQuery` and `Observations` structs are examples of data structures that
//! are carried by messages.
//!
//! The `create_init_lifecycle` function creates a pair of envelopes that are used
//! to bootstrap the lifecycle of an actor. One envelope instructs the actor to enter
//! initialization mode and replay its event history to recalculate its state. The
//! other envelope instructs the persistence system to look up and deliver the previous
//! events to the new instance of the actor.
//!
//! The module also provides types for handling errors that may arise during the
//! communication process (`NvError` and `NvResult<T>`), as well as a type used to
//! hint at the intent of a `Message<T>` (`MtHint`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type NvResult<T> = Result<T, NvError>;

#[derive(Debug, Clone)]
pub struct NvError {
    pub reason: String,
}

impl fmt::Display for NvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathQuery {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Observations {
    pub datetime: String,
    pub values: HashMap<i32, f64>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneMapping {
    pub path: String,
    pub gene_type: String,
}

/// all actor messages are delivered in envelops that contain optional
/// sender objects - these are set when a `tell` message is sent so that
/// the reply can be delivered.  These replies are not placed in envelopes.
#[derive(Debug)]
pub struct Envelope<T> {
    pub message: Message<T>,
    pub respond_to: Option<oneshot::Sender<NvResult<Message<T>>>>,
    pub datetime: OffsetDateTime,
    pub stream_to: Option<mpsc::Sender<Message<T>>>,
    pub stream_from: Option<mpsc::Receiver<Message<T>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MtHint {
    Update,
    Query,
    GeneMapping,
}

impl fmt::Display for MtHint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = match self {
            Self::Query => "query",
            Self::Update => "update",
            Self::GeneMapping => "gene mapping",
        };
        write!(f, "[{display_text}]")
    }
}

/// all actor API interaction is via async messages
#[derive(Debug, Clone)]
pub enum Message<T> {
    /// 'Query' is usually the 'ask' payload.  
    Query { path: String },
    /// 'Update' is usually the 'tell' payload
    Update {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, T>,
    },
    /// the response to most Query/ask interactions
    StateReport {
        datetime: OffsetDateTime,
        path: String,
        values: HashMap<i32, T>,
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
    LoadCmd { path: String },
    /// ReadAllCmd and PrintOneCmd orchestrate reads from stdin and writes to
    /// stdout in cli use cases
    ReadAllCmd {},
    Content {
        text: String,
        hint: MtHint,
        path: Option<String>,
    },
}

impl<T> fmt::Display for Envelope<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = format!("env: {} - {}", self.datetime, self.message);
        write!(f, "{display_text}")
    }
}

impl<T> fmt::Display for Message<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = match self {
            Self::Content { text, hint, path } => path.clone().map_or_else(
                || format!("[Content {hint}:{text}]"),
                |path| format!("[Content {path} {hint}:{text}]"),
            ),
            Self::LoadCmd { path } => format!("[LoadCmd {path}]"),
            Self::ReadAllCmd {} => "[ReadAllCmd]".to_string(),
            Self::InitCmd {} => "[InitCmd]".to_string(),
            Self::EndOfStream {} => "[EndOfStream]".to_string(),
            Self::StateReport { .. } => "[StateReport]".to_string(),
            Self::Update { .. } => "[Update]".to_string(),
            Self::Query { .. } => "[Query]".to_string(),
        };
        write!(f, "{display_text}")
    }
}

impl<T> Default for Envelope<T> {
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

struct LifeCycleBuilder<T> {
    load_from: Option<mpsc::Receiver<Message<T>>>,
    send_to: Option<mpsc::Sender<Message<T>>>,
    send_to_path: Option<String>,
    respond_to: Option<oneshot::Sender<NvResult<Message<T>>>>,
}

impl<T> LifeCycleBuilder<T> {
    const fn new() -> Self {
        Self {
            load_from: None,
            send_to: None,
            send_to_path: None,
            respond_to: None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    fn with_respond_to(mut self, respond_to: oneshot::Sender<NvResult<Message<T>>>) -> Self {
        self.respond_to = Some(respond_to);
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    fn with_load_from(mut self, load_from: mpsc::Receiver<Message<T>>) -> Self {
        self.load_from = Some(load_from);
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    fn with_send_to(mut self, send_to: mpsc::Sender<Message<T>>, send_to_path: String) -> Self {
        self.send_to = Some(send_to);
        self.send_to_path = Some(send_to_path);
        self
    }

    fn build(self) -> (Envelope<T>, Envelope<T>) {
        (
            Envelope {
                datetime: OffsetDateTime::now_utc(),
                respond_to: self.respond_to,
                stream_from: self.load_from,
                stream_to: None,
                message: Message::InitCmd {},
            },
            Envelope {
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
pub fn create_init_lifecycle<T>(
    path: String,
    bufsz: usize,
    respond_to: oneshot::Sender<NvResult<Message<T>>>,
) -> (Envelope<T>, Envelope<T>) {
    let (tx, rx) = mpsc::channel(bufsz);
    let builder = LifeCycleBuilder::new()
        .with_load_from(rx)
        .with_send_to(tx, path)
        .with_respond_to(respond_to);
    builder.build()
}
