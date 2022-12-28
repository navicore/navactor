use crate::messages::ActorMessage;
use tokio::sync::{mpsc, oneshot};

struct StdoutActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

impl StdoutActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        StdoutActor { receiver }
    }
    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::PrintOneCmd { text } => println!("{}", text),
            ActorMessage::IsCompleteMsg { respond_to_opt } => match respond_to_opt {
                Some(respond_to) => {
                    let _ = respond_to.send(ActorMessage::IsCompleteMsg {
                        respond_to_opt: None,
                    });
                }
                _ => {}
            },
            _ => println!(""),
        }
    }
}

async fn acting(mut actor: StdoutActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct StdoutActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl StdoutActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdoutActor::new(receiver);
        tokio::spawn(acting(actor));
        Self { sender }
    }

    pub async fn print(&self, text: String) {
        let msg = ActorMessage::PrintOneCmd { text };
        let _ = self.sender.send(msg).await;
    }

    pub async fn complete(&self, respond_to: oneshot::Sender<ActorMessage>) {
        let msg = ActorMessage::IsCompleteMsg {
            respond_to_opt: Some(respond_to),
        };
        let _ = self.sender.send(msg).await;
    }
}
