use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::stdout_actor::StdoutActorHandle;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<MessageEnvelope>,
    output: StdoutActorHandle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => {
                if let Message::ReadAllCmd {} = message {
                    let mut lines = BufReader::new(stdin()).lines();

                    while let Some(text) = lines.next_line().await.expect("failed to read stream") {
                        let msg = Message::PrintOneCmd { text };
                        self.output.tell(msg).await
                    }

                    // forward the respond_to handle so that the output actor can respond when all
                    // is printed
                    let complete_msg = Message::IsCompleteMsg {};
                    let senv = MessageEnvelope {
                        message: complete_msg,
                        respond_to_opt,
                    };
                    self.output.send(senv).await
                } else {
                    log::warn!("unexpected: {:?}", message);
                }
            }
        }
    }
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
}

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
    pub async fn read(&self) -> Message {
        let msg = Message::ReadAllCmd {};
        self.ask(msg).await
    }
}
