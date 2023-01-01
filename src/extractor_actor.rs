use crate::actor::Actor;
use crate::messages::ActorMessage;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

struct ExtractorActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

#[async_trait]
impl Actor for ExtractorActor {
    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::DefineCmd {
                spec,
                respond_to_opt: Some(respond_to),
            } => {
                log::debug!("defining spec {}", spec);
                // TODO
                //
                //
                let complete_msg = ActorMessage::IsCompleteMsg {
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
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        ExtractorActor { receiver }
    }
}

async fn acting(mut actor: ExtractorActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct ExtractorActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl ExtractorActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = ExtractorActor::new(receiver);
        tokio::spawn(acting(actor));
        Self { sender }
    }

    pub async fn define(&self, spec: String) -> ActorMessage {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::DefineCmd {
            spec,
            respond_to_opt: Some(send),
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}
