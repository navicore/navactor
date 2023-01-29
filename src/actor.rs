use crate::message::create_init_lifecycle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use std::fmt;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type ActorResult<T> = std::result::Result<T, ActorError>;

#[derive(Debug, Clone)]
pub struct ActorError {
    reason: String,
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unsuccessful operation: {}", self.reason)
    }
}

/// all actors must implement this trait
#[async_trait]
pub trait Actor {
    /// the function to implement per actor
    async fn handle_envelope(&mut self, envelope: MessageEnvelope);
    async fn stop(&mut self);
}

/// ActorHandle is the API for all actors
pub struct ActorHandle {
    #[doc(hidden)]
    pub sender: mpsc::Sender<MessageEnvelope>,
}

/// ActorHandle is the API for all actors via `ask` and `tell`
impl<'a> ActorHandle {
    // INTERNAL: currently used by builtins (nv actors) implementing
    // actors that forward respond_to in workflows.
    #[doc(hidden)]
    pub async fn send(&self, envelope: MessageEnvelope) -> ActorResult<()> {
        self.sender
            .send(envelope)
            .await
            //.expect("actor cannot receive");
            .map_err(|e| ActorError {
                reason: e.to_string(),
            })
    }

    /// fire and forget
    pub async fn tell(&self, message: Message) -> ActorResult<()> {
        let envelope = MessageEnvelope {
            message,
            respond_to: None,
            ..Default::default()
        };

        self.send(envelope).await
    }

    /// request <-> response
    pub async fn ask(&self, message: Message) -> ActorResult<Message> {
        let (send, recv) = oneshot::channel();

        let envelope = MessageEnvelope {
            message,
            respond_to: Some(send),
            ..Default::default()
        };

        log::trace!("sending envelope");
        match self.send(envelope).await {
            Ok(_) => recv.await.map_err(|e| ActorError {
                reason: e.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    pub async fn integrate(&self, path: String, helper: &ActorHandle) -> ActorResult<Message> {
        let (send, recv) = oneshot::channel();

        let (init_cmd, load_cmd) = create_init_lifecycle(path, 8, send);

        helper.send(load_cmd).await.expect("cannot send");

        self.send(init_cmd).await.expect("cannot send");

        recv.await.map_err(|e| ActorError {
            reason: e.to_string(),
        })
    }

    // ActorHandle constructor is an internal API use in the convenience functions
    // of the various per-actor ActorHandle impls
    #[doc(hidden)]
    pub fn new(sender: mpsc::Sender<MessageEnvelope>) -> Self {
        Self { sender }
    }
}
