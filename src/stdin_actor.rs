use crate::actor::Actor;
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

            while let Some(line) = lines.next_line().await.expect("failed to read stream") {
                self.output.print(line).await
            }

            self.output
                .complete(respond_to)
                .await
                .expect("can not send EOF report");
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

async fn acting(mut actor: StdinActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

#[derive(Clone)]
pub struct StdinActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl StdinActorHandle {
    pub fn new(bufsz: usize, output: StdoutActorHandle) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdinActor::new(receiver, output);
        tokio::spawn(acting(actor));

        Self { sender }
    }

    pub async fn read(&self) -> ActorMessage {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::ReadAllCmd {
            respond_to_opt: Some(send),
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
