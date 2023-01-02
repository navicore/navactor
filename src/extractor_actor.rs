use crate::actor::Actor;
use crate::actor::ActorHandle;
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

// actor private constructor
impl ExtractorActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        ExtractorActor { receiver }
    }
}

/// public interface - you get a handle back, not the actual actor
pub fn new(bufsz: usize) -> ActorHandle {
    async fn start(mut actor: ExtractorActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = ExtractorActor::new(receiver);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
