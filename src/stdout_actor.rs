use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

struct StdoutActor {
    receiver: mpsc::Receiver<MessageEnvelope>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => match message {
                Message::PrintOneCmd { text } => println!("{}", text),
                Message::IsCompleteMsg {} => {
                    if let Some(respond_to) = respond_to_opt {
                        let complete_msg = Message::IsCompleteMsg {};
                        respond_to
                            .send(complete_msg)
                            .expect("could not send completion token");
                    }
                }
                _ => {
                    log::warn!("unexpected: {:?}", message);
                }
            },
        }
    }
}

impl StdoutActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        StdoutActor { receiver }
    }
}

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
