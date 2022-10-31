use crate::messages::ActorMessage;
use tokio::sync::{mpsc, oneshot};

struct StdoutActor {
    receiver: mpsc::Receiver<ActorMessage>,
}

impl StdoutActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        StdoutActor { receiver }
    }
    async fn handle_message(&mut self, msg: ActorMessage) {
        println!("output got msg............");
        match msg {
            ActorMessage::PrintOneCmd { text } => println!("{}", text),
            ActorMessage::IsCompleteMsg { respond_to } => {
                let _ = respond_to.send(1);
            }
            _ => println!(""),
        }
    }
}

async fn run_my_actor(mut actor: StdoutActor) {
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
        tokio::spawn(run_my_actor(actor));
        Self { sender }
    }

    pub async fn print(&self, text: String) {
        println!("got print");
        let msg = ActorMessage::PrintOneCmd { text };
        let _ = self.sender.send(msg).await;
    }

    pub async fn complete(&self, respond_to: oneshot::Sender<u32>) {
        let msg = ActorMessage::IsCompleteMsg { respond_to };
        let _ = self.sender.send(msg).await;
    }
}
