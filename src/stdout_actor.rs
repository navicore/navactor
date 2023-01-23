use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// in CLI mode, printing to stdout is helpful and can enable `nv` to be used
/// in combination with other *nix tools.
pub struct StdoutActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn stop(&mut self) {}
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            ..
        } = envelope;
        match message {
            Message::PrintOneCmd { text } => println!("{}", text),
            Message::StateReport { path, values, .. } => {
                println!("{} current state: {:?}", path, values)
            }
            Message::EndOfStream {} => {
                if let Some(respond_to) = respond_to {
                    let complete_msg = Message::EndOfStream {};
                    respond_to
                        .send(complete_msg)
                        .expect("could not send completion token");
                }
            }
            _ => {
                log::warn!("unexpected: {:?}", message);
            }
        }
    }
}

/// actor private constructor
impl StdoutActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        StdoutActor { receiver }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize) -> ActorHandle {
    async fn start(mut actor: StdoutActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StdoutActor::new(receiver);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
