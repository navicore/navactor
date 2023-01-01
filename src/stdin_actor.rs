use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::stdout_actor::StdoutActorHandle;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<Message>,
    output: StdoutActorHandle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_message(&mut self, msg: Message) {
        if let Message::ReadAllCmd {
            respond_to_opt: Some(respond_to),
        } = msg
        {
            let mut lines = BufReader::new(stdin()).lines();

            while let Some(text) = lines.next_line().await.expect("failed to read stream") {
                let msg = Message::PrintOneCmd { text };
                self.output.tell(msg).await
            }

            let complete_msg = Message::IsCompleteMsg {
                respond_to_opt: Some(respond_to),
            };
            self.output.tell(complete_msg).await
        } else {
            log::warn!("unexpected: {:?}", msg);
        }
    }
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<Message>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
}

pub struct StdinActorHandle {
    sender: mpsc::Sender<Message>,
}

#[async_trait]
impl ActorHandle for StdinActorHandle {
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
    //TODO need to make read generic for "send and get reply" of any command
    //TODO need to make read generic for "send and get reply" of any command
    //TODO need to make read generic for "send and get reply" of any command
    //TODO need to make read generic for "send and get reply" of any command
    //TODO need to make read generic for "send and get reply" of any command
    pub async fn read(&self) -> Message {
        let (send, recv) = oneshot::channel();
        let msg = Message::ReadAllCmd {
            respond_to_opt: Some(send),
        };
        let _ = self.ask(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
