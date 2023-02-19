//!This module is responsible for handling messages to be printed to the standard output stream.
//!
//!In `CLI` mode, `nv` can be used with other *nix tools and in such cases, printing to `stdout`
//!can be beneficial. The module implements the `Actor` trait and defines its own `handle_envelope`
//!and stop methods.
//!
//!It receives a channel of type `mpsc::Receiver<Envelope<f64>>` to receive messages to print, and
//!it pattern matches on the type of the incoming message.
//!
//!If the message is a `TextMsg`, it prints the message to the standard output. If the message is a
//!`StateReport` or an `Update`, it prints the appropriate message with the path and values.
//!
//!When a message of type `EndOfStream` is received, it sends the message to the stream creator via
//!`respond_to` if there is any.
//!
//!The module has a public constructor function `new` that returns a `Handle` to interact with the
//!actor.

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
    pub receiver: mpsc::Receiver<Envelope<f64>>,
}

#[async_trait]
impl Actor for StdoutActor {
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
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
                        .unwrap_or_else(|e| log::error!("cannot respond to ask: {e:?}"));
                }
            }
            _ => {
                log::warn!("unexpected: {message}");
            }
        }
    }
    async fn stop(&self) {}
}

/// actor private constructor
impl StdoutActor {
    const fn new(receiver: mpsc::Receiver<Envelope<f64>>) -> Self {
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
