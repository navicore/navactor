use crate::fulllines::FullLines;
use std::io;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<StdinActorMessage>,
}
enum StdinActorMessage {
    ReadCmd { respond_to: oneshot::Sender<u32> },
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<StdinActorMessage>) -> Self {
        StdinActor { receiver }
    }
    fn handle_message(&mut self, msg: StdinActorMessage) {
        match msg {
            StdinActorMessage::ReadCmd { respond_to } => {
                for line_result in io::stdin().lock().full_lines() {
                    match line_result {
                        Ok(line) => print!("{}", line),
                        _ => println!(),
                    }
                }

                let _ = respond_to.send(1);
            }
        }
    }
}

async fn run_my_actor(mut actor: StdinActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct StdinActorHandle {
    sender: mpsc::Sender<StdinActorMessage>,
}

impl StdinActorHandle {
    pub fn new(bufsz: usize) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdinActor::new(receiver);
        tokio::spawn(run_my_actor(actor));

        Self { sender }
    }

    pub async fn read(&self) -> u32 {
        let (send, recv) = oneshot::channel();
        let msg = StdinActorMessage::ReadCmd { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("StdinActor task has been killed")
    }
}
