use crate::actor::Actor;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct StdoutActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => match message {
                Message::PrintOneCmd { text } => println!("{}", text),
                Message::IsCompleteMsg {} => {
                    if let Some(respond_to) = respond_to_opt {
                        let complete_msg = Message::IsCompleteMsg {};
                        respond_to
                            .send(complete_msg)
                            .expect("could not send completion token");
                    }
                }
                _ => {
                    log::warn!("unexpected: {:?}", message);
                }
            },
        }
    }
}

impl StdoutActor {
    pub fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        StdoutActor { receiver }
    }
}
