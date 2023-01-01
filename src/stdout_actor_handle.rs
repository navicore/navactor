use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::stdout_actor::StdoutActor;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

pub struct StdoutActorHandle {
    sender: mpsc::Sender<MessageEnvelope>,
}

#[async_trait]
impl ActorHandle for StdoutActorHandle {
    async fn send(&self, envelope: MessageEnvelope) {
        self.sender
            .send(envelope)
            .await
            .expect("actor handle can not send");
    }
    async fn tell(&self, message: Message) {
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: None,
        };
        self.send(envelope).await;
    }
    async fn ask(&self, message: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: Some(send),
        };
        let _ = self.send(envelope).await;
        recv.await.expect("StdinActor task has been killed")
    }
}

impl StdoutActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdoutActor::new(receiver);
        tokio::spawn(StdoutActorHandle::start(actor));
        Self { sender }
    }
    async fn start(mut actor: StdoutActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
}
