use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

struct StdoutActor {
    receiver: mpsc::Receiver<Message>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::PrintOneCmd { text } => println!("{}", text),
            Message::IsCompleteMsg { respond_to_opt } => {
                if let Some(respond_to) = respond_to_opt {
                    let complete_msg = Message::IsCompleteMsg {
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
    fn new(receiver: mpsc::Receiver<Message>) -> Self {
        StdoutActor { receiver }
    }
}

pub struct StdoutActorHandle {
    sender: mpsc::Sender<Message>,
}

#[async_trait]
impl ActorHandle for StdoutActorHandle {
    async fn tell(&self, msg: Message) {
        self.sender
            .send(msg)
            .await
            .expect("actor handle can not send");
    }
    async fn ask(&self, msg: Message) -> Message {
        let (send, recv) = oneshot::channel();
        let _ = self.tell(msg).await;
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
        while let Some(msg) = actor.receiver.recv().await {
            actor.handle_message(msg).await;
        }
    }
}
