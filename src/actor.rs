use crate::message::create_init_lifecycle;
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
            respond_to: None,
            ..Default::default()
        };
        self.send(envelope).await;
    }
    /// request <-> response
    pub async fn ask(&self, message: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let envelope = MessageEnvelope {
            message,
            respond_to: Some(send),
            ..Default::default()
        };
        self.send(envelope).await;
        recv.await.expect("other actor cannot reply")
    }
    /// set up a stream between two actors and the result will be the result of
    /// the second actor processing next_message.
    ///
    /// create a pair of MessageEnvelopes.  
    ///
    /// The first one has an InitCmd as message, a receiver, and a reply_to
    /// that returns the result of the message to the integrate caller once
    /// the 'last_message' and then the EOS message arrives.
    ///
    /// The second one has a LoadCmd as message to tell the helper to read the
    /// events for this path and write them to the send obj in the envelope.  After
    /// the last jrnl store message is written, send the "next_message" and
    /// then send an EOS.
    ///
    /// NOTE path is on the function because not all actors know their path.
    /// TODO: make this a state_actor-only function
    ///
    /// this is first used to enable the resurrection of actors
    pub async fn integrate(&self, message: Message, path: String, helper: &ActorHandle) -> Message {
        let (send, recv) = oneshot::channel();

        let (init_cmd, load_cmd) = create_init_lifecycle(message.clone(), path, 8, Some(send));

        helper.send(load_cmd).await;

        self.send(init_cmd).await;

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
