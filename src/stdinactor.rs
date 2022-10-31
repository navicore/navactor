use crate::lineiter::LineIterator;
use crate::messages::ActorMessage;
use crate::stdoutactor::StdoutActorHandle;
use std::io;
use tokio::sync::{mpsc, oneshot};

struct StdinActor {
    receiver: mpsc::Receiver<ActorMessage>,
    output: StdoutActorHandle,
}

impl StdinActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
    ///////////////// NOTE the BUG is that there are 2 futures in this
    /// non-async method and you can not pass a future across threads - ie: spawn
    /// non-async method and you can not pass a future across threads - ie: spawn
    /// non-async method and you can not pass a future across threads - ie: spawn
    /// non-async method and you can not pass a future across threads - ie: spawn
    /// non-async method and you can not pass a future across threads - ie: spawn
    ///////////////// NOTE
    fn handle_message(&mut self, msg: ActorMessage) {
        println!("handle msg...");
        match msg {
            ActorMessage::ReadAllCmd { respond_to } => {
                println!("starting reading stdin...");
                for line_result in io::stdin().lock().line_iter() {
                    match line_result {
                        Ok(line) => {
                            let _ = self.output.print(line);
                        }
                        _ => println!(""),
                    }
                }
                println!("...stopping reading stdin");
                let _ = self.output.complete(respond_to);
            }
            e => println!("unexpected: {:?}", e),
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
    sender: mpsc::Sender<ActorMessage>,
}

impl StdinActorHandle {
    pub fn new(bufsz: usize, output: StdoutActorHandle) -> Self {
        let (sender, receiver) = mpsc::channel(bufsz);
        let actor = StdinActor::new(receiver, output);
        tokio::spawn(run_my_actor(actor));

        Self { sender }
    }

    pub async fn read(&self) -> u32 {
        println!("ejs 1");
        let (send, recv) = oneshot::channel();
        println!("ejs 2");
        let msg = ActorMessage::ReadAllCmd { respond_to: send };
        println!("ejs 3");
        let _ = self.sender.send(msg).await;
        println!("ejs 4");

        use std::{thread, time};

        let thou_millis = time::Duration::from_millis(1000);

        thread::sleep(thou_millis);

        recv.await.expect("StdinActor task has been killed")
    }
}
