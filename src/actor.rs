use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// all actors must implement this trait
#[async_trait]
pub trait Actor {
    /// the function to implement per actor
    async fn handle_envelope(&mut self, envelope: MessageEnvelope);
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
    pub async fn send(&self, envelope: MessageEnvelope) {
        self.sender
            .send(envelope)
            .await
            .expect("other actor cannot receive");
    }
    /// fire and forget
    pub async fn tell(&self, message: Message) {
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: None,
            ..Default::default()
        };
        self.send(envelope).await;
    }
    /// request <-> response
    pub async fn ask(&self, message: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: Some(send),
            ..Default::default()
        };
        self.send(envelope).await;
        recv.await.expect("other actor cannot reply")
    }
}

/// ActorHandle is the only API for actors.  ActorHandle(s) may be passed
/// around like erlang pids
impl ActorHandle {
    // ActorHandle constructor is an internal API use in the convenience functions
    // of the various per-actor ActorHandle impls
    #[doc(hidden)]
    pub fn new(sender: mpsc::Sender<MessageEnvelope>) -> Self {
        Self { sender }
    }
}
