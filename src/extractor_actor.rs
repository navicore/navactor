use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::Respondable;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

struct ExtractorActor {
    receiver: mpsc::Receiver<Message>,
}

#[async_trait]
impl Actor for ExtractorActor {
    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::DefineCmd {
                spec,
                respond_to_opt: Some(respond_to),
            } => {
                log::debug!("defining spec {}", spec);
                // TODO
                //
                //
                let complete_msg = Message::IsCompleteMsg {
                    respond_to_opt: None,
                };
                respond_to
                    .send(complete_msg)
                    .expect("could not send completion token");
            }
            _ => {
                log::warn!("unexpected: {:?}", msg);
            }
        }
    }
}

impl ExtractorActor {
    fn new(receiver: mpsc::Receiver<Message>) -> Self {
        ExtractorActor { receiver }
    }
}

pub struct ExtractorActorHandle {
    sender: mpsc::Sender<Message>,
}

#[async_trait]
impl ActorHandle for ExtractorActorHandle {
    async fn tell(&self, msg: Message) {
        self.sender
            .send(msg)
            .await
            .expect("actor handle can not send");
    }
    async fn ask(&self, msg: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let _ = self.ask(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}

impl ExtractorActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = ExtractorActor::new(receiver);
        tokio::spawn(ExtractorActorHandle::start(actor));
        Self { sender }
    }
    async fn start(mut actor: ExtractorActor) {
        while let Some(msg) = actor.receiver.recv().await {
            actor.handle_message(msg).await;
        }
    }

    //TODO need to make read generic for "send and get reply" of any command
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    //TODO or is this tell and ask?
    pub async fn define(&self, spec: String) -> Message {
        let (send, recv) = oneshot::channel();
        let msg = Message::DefineCmd {
            spec,
            respond_to_opt: Some(send),
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}
