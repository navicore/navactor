//!This module implements the `StdinActor`, which is responsible for reading commands from the
//!standard input stream in command-line interface (`CLI`) mode. When a `ReadAllCmd` message is
//!received, the actor reads all incoming lines from the standard input stream until the stream is
//!closed, sending a `TextMsg` message with the received text to the output handle for each line.
//!Once the end of the stream is reached, a `EndOfStream` message is sent to the next hop to
//!trigger any necessary cleanup and shutdown. This actor is only used in `CLI` mode and is used to
//!interact with the command-line interface by reading input commands from the user.

use crate::actors::actor::Actor;
use crate::actors::actor::Handle;
use crate::actors::message::Envelope;
use crate::actors::message::Message;
use crate::actors::message::MtHint;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;
use tracing::error;
use tracing::warn;

/// the stdin actor is only used in CLI mode.  it gets a single command to
/// read from stdin and it reads until the EOF.  once it sees EOF, it sends
/// a `EndOfStream` msg to the next hop to trigger any cleanup and shutdown.
pub struct StdinActor {
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub output: Handle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        let Envelope {
            message,
            respond_to,
            ..
        } = envelope;

        if matches!(message, Message::ReadAllCmd {}) {
            let mut lines = BufReader::new(stdin()).lines();

            while let Some(text) = lines.next_line().await.unwrap_or_else(|e| {
                error!("failed to read stream: {e:?}");
                None
            }) {
                let hint = if text.contains("gene_type") {
                    MtHint::GeneMapping
                } else {
                    MtHint::Update
                };
                let msg = Message::Content {
                    text,
                    hint,
                    path: None,
                };
                match self.output.tell(msg).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("cannot send message: {e:?}");
                        return;
                    }
                }
            }

            let complete_msg = Message::EndOfStream {};

            let senv = Envelope {
                message: complete_msg,
                respond_to,
                ..Default::default()
            };

            match self.output.send(senv).await {
                Ok(()) => {}
                Err(e) => {
                    error!("cannot send end-of-stream message: {e:?}");
                }
            }
        } else {
            warn!("unexpected: {message}");
        }
    }
    async fn stop(&self) {}
    async fn start(&mut self) {}
}

/// actor private constructor
impl StdinActor {
    const fn new(receiver: mpsc::Receiver<Envelope<f64>>, output: Handle) -> Self {
        Self { receiver, output }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(bufsz: usize, output: Handle) -> Handle {
    async fn start(mut actor: StdinActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = StdinActor::new(receiver, output);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    actor_handle
}
