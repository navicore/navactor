use crate::message::create_init_lifecycle;
use crate::message::ActorError;
use crate::message::ActorResult;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type ActorState<T> = std::collections::HashMap<i32, T>;

/// all actors must implement this trait
#[async_trait]
pub trait Actor {
    /// the function to implement per actor
    async fn handle_envelope(&mut self, envelope: MessageEnvelope);
    async fn stop(&self);
}

/// `ActorHandle` is the API for all actors
pub struct ActorHandle {
    #[doc(hidden)]
    pub sender: mpsc::Sender<MessageEnvelope>,
}

/// `ActorHandle` is the API for all actors via `ask` and `tell`
impl<'a> ActorHandle {
    // INTERNAL: currently used by builtins (nv actors) implementing
    // actors that forward respond_to in workflows.
    #[doc(hidden)]
    pub async fn send(&self, envelope: MessageEnvelope) -> ActorResult<()> {
        self.sender.send(envelope).await.map_err(|e| ActorError {
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

        log::trace!("tell sending envelope {envelope:?}");
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

        log::trace!("ask sending envelope: {envelope:?}");
        match self.send(envelope).await {
            Ok(_) => recv.await.map_err(|e| ActorError {
                reason: e.to_string(),
            })?,

            Err(e) => Err(e),
        }
    }

    /// call to coordinate the instantiation of a new acotr with the help
    /// of another actor - usually a datastore journal service
    pub async fn integrate(&self, path: String, helper: &Self) -> ActorResult<Message> {
        let (send, recv): (
            oneshot::Sender<ActorResult<Message>>,
            oneshot::Receiver<ActorResult<Message>>,
        ) = oneshot::channel();

        let (init_cmd, load_cmd) = create_init_lifecycle(path, 8, send);

        helper.send(load_cmd).await.map_err(|e| ActorError {
            reason: e.to_string(),
        })?;

        self.send(init_cmd).await.map_err(|e| ActorError {
            reason: e.to_string(),
        })?;

        recv.await.map_err(|e| ActorError {
            reason: e.to_string(),
        })?
    }

    // ActorHandle constructor is an internal API use in the convenience functions
    // of the various per-actor ActorHandle impls
    #[doc(hidden)]
    #[must_use]
    pub const fn new(sender: mpsc::Sender<MessageEnvelope>) -> Self {
        Self { sender }
    }
}
