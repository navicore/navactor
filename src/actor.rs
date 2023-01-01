use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[async_trait]
pub trait Actor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope);
}

pub struct ActorHandle {
    pub sender: mpsc::Sender<MessageEnvelope>,
}

impl ActorHandle {
    // system message but it is currently used by userland code implementing
    // actors that forward respond_to in workflows.  TODO for a way to do this w/o the
    // app code touching or seeing the envelope or mpsc objects
    pub async fn send(&self, envelope: MessageEnvelope) {
        self.sender
            .send(envelope)
            .await
            .expect("actor handle can not send");
    }
    pub async fn tell(&self, message: Message) {
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: None,
        };
        self.send(envelope).await;
    }
    pub async fn ask(&self, message: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: Some(send),
        };
        let _ = self.send(envelope).await;
        recv.await.expect("StdinActor task has been killed")
    }
}

impl ActorHandle {
    pub fn new(sender: mpsc::Sender<MessageEnvelope>) -> Self {
        Self { sender }
    }
}
