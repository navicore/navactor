use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::stdout_actor_handle::StdoutActorHandle;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;

pub struct StdinActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: StdoutActorHandle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => {
                if let Message::ReadAllCmd {} = message {
                    let mut lines = BufReader::new(stdin()).lines();

                    while let Some(text) = lines.next_line().await.expect("failed to read stream") {
                        let msg = Message::PrintOneCmd { text };
                        self.output.tell(msg).await
                    }

                    // forward the respond_to handle so that the output actor can respond when all
                    // is printed
                    let complete_msg = Message::IsCompleteMsg {};
                    let senv = MessageEnvelope {
                        message: complete_msg,
                        respond_to_opt,
                    };
                    self.output.send(senv).await
                } else {
                    log::warn!("unexpected: {:?}", message);
                }
            }
        }
    }
}

impl StdinActor {
    pub fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: StdoutActorHandle) -> Self {
        StdinActor { receiver, output }
    }
}
