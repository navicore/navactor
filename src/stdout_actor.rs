use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// in CLI mode, printing to stdout is helpful and can enable `nv` to be used
/// in combination with other *nix tools.
pub struct StdoutActor<'a> {
    pub receiver: mpsc::Receiver<MessageEnvelope<'a>>,
}

#[async_trait]
impl<'a> Actor<'a> for StdoutActor<'a> {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope<'a>) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
                timestamp: _,
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

/// actor private constructor
impl<'a> StdoutActor<'a> {
    fn new(receiver: mpsc::Receiver<MessageEnvelope<'a>>) -> Self {
        StdoutActor { receiver }
    }
}

/// actor handle public constructor
pub fn new<'a>(bufsz: usize) -> ActorHandle<'static> {
    async fn start<'a>(mut actor: StdoutActor<'static>) {
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
