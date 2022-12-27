use crate::messages::ActorMessage;
use crate::stdoutactor::StdoutActorHandle;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<ActorMessage>,
    output: StdoutActorHandle,
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::ReadAllCmd { respond_to } => {
                let mut lines = BufReader::new(stdin()).lines();

                while let Some(line) = lines.next_line().await.expect("failed to read stream") {
                    let _ = self.output.print(line).await;
                }

                let _ = self.output.complete(respond_to).await; // THIS DOES NOT WORK
            }
            e => println!("unexpected: {:?}", e),
        }
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

    pub async fn read(&self) -> u32 {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::ReadAllCmd { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
