use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::messages::ActorMessage;
use async_trait::async_trait;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, oneshot};

struct StdoutActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::PrintOneCmd { text } => println!("{}", text),
            ActorMessage::IsCompleteMsg { respond_to_opt } => {
                if let Some(respond_to) = respond_to_opt {
                    let complete_msg = ActorMessage::IsCompleteMsg {
                        respond_to_opt: None,
                    };
                    respond_to
                        .send(complete_msg)
                        .expect("could not send completion token");
                }
            }
            _ => {
                log::warn!("unexpected: {:?}", msg);
            }
        }
    }
}

impl StdoutActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        StdoutActor { receiver }
    }
}

async fn acting(mut actor: StdoutActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct StdoutActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

#[async_trait]
impl ActorHandle for StdoutActorHandle {
    async fn send(&self, msg: ActorMessage) {
        self.sender
            .send(msg)
            .await
            .expect("actor handle can not send");
    }
}

impl StdoutActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdoutActor::new(receiver);
        tokio::spawn(acting(actor));
        Self { sender }
    }

    pub async fn complete(
        &self,
        respond_to: oneshot::Sender<ActorMessage>,
    ) -> Result<(), SendError<ActorMessage>> {
        let msg = ActorMessage::IsCompleteMsg {
            respond_to_opt: Some(respond_to),
        };
        self.sender.send(msg).await
    }
}
