use crate::actor::Actor;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct ExtractorActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
}

#[async_trait]
impl Actor for ExtractorActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => {
                match message {
                    Message::DefineCmd { spec } => {
                        log::debug!("defining spec {}", spec);
                        // TODO
                        //
                        //
                        let complete_msg = Message::IsCompleteMsg {};
                        match respond_to_opt {
                            Some(respond_to) => {
                                respond_to
                                    .send(complete_msg)
                                    .expect("could not send completion token");
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        log::warn!("unexpected: {:?}", message);
                    }
                }
            }
        }
    }
}

impl ExtractorActor {
    pub fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        ExtractorActor { receiver }
    }
}
