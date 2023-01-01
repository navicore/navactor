use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::messages::ActorMessage;
use crate::stdout_actor::StdoutActorHandle;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<ActorMessage>,
    output: StdoutActorHandle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_message(&mut self, msg: ActorMessage) {
        if let ActorMessage::ReadAllCmd {
            respond_to_opt: Some(respond_to),
        } = msg
        {
            let mut lines = BufReader::new(stdin()).lines();

            while let Some(text) = lines.next_line().await.expect("failed to read stream") {
                let msg = ActorMessage::PrintOneCmd { text };
                self.output.send(msg).await
            }

            let complete_msg = ActorMessage::IsCompleteMsg {
                respond_to_opt: Some(respond_to),
            };
            self.output.send(complete_msg).await
        } else {
            log::warn!("unexpected: {:?}", msg);
        }
    }
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
}

pub struct StdinActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

#[async_trait]
impl ActorHandle for StdinActorHandle {
    async fn send(&self, msg: ActorMessage) {
        self.sender
            .send(msg)
            .await
            .expect("actor handle can not send");
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
        while let Some(msg) = actor.receiver.recv().await {
            actor.handle_message(msg).await;
        }
    }
    pub async fn read(&self) -> ActorMessage {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::ReadAllCmd {
            respond_to_opt: Some(send),
        };
        let _ = self.send(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
