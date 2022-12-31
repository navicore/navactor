use crate::messages::ActorMessage;
use tokio::sync::{mpsc, oneshot};

struct ExtractorActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

impl ExtractorActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        ExtractorActor { receiver }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::DefineCmd { spec, respond_to } => {
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

async fn acting(mut actor: ExtractorActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
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
            respond_to: send,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
