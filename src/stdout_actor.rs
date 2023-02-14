use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::Envelope;
use crate::message::Message;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// in CLI mode, printing to stdout is helpful and can enable `nv` to be used
/// in combination with other *nix tools.
pub struct StdoutActor {
    pub receiver: mpsc::Receiver<Envelope>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn stop(&self) {}
    async fn handle_envelope(&mut self, envelope: Envelope) {
        let Envelope {
            message,
            respond_to,
            ..
        } = envelope;

        match &message {
            Message::TextMsg { text, hint: _ } => println!("{text}"),
            Message::StateReport { path, values, .. } => {
                println!("{path} current state: {values:?}");
                respond_or_log_error(respond_to, Ok(message));
            }
            Message::Update { path, values, .. } => {
                println!("{path} new observations: {values:?}");
                respond_or_log_error(respond_to, Ok(message));
            }
            Message::EndOfStream {} => {
                if let Some(respond_to) = respond_to {
                    respond_to
                        .send(Ok(Message::EndOfStream {}))
                        .unwrap_or_else(|e| log::error!("cannot respond to ask: {:?}", e));
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
    const fn new(receiver: mpsc::Receiver<Envelope>) -> Self {
        Self { receiver }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(bufsz: usize) -> Handle {
    async fn start(mut actor: StdoutActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = StdoutActor::new(receiver);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    actor_handle
}
