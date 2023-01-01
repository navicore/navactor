use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::stdin_actor::StdinActor;
use crate::stdout_actor_handle::StdoutActorHandle;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

pub struct StdinActorHandle {
    sender: mpsc::Sender<MessageEnvelope>,
}

#[async_trait]
impl ActorHandle for StdinActorHandle {
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

impl StdinActorHandle {
    pub fn new(bufsz: usize, output: StdoutActorHandle) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdinActor::new(receiver, output);
        tokio::spawn(StdinActorHandle::start(actor));
        Self { sender }
    }
    async fn start(mut actor: StdinActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
}
