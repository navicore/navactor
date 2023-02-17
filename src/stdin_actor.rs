use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::MtHint;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;

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
                log::error!("failed to read stream: {e:?}");
                None
            }) {
                let msg = Message::TextMsg {
                    text,
                    hint: MtHint::Update,
                };
                match self.output.tell(msg).await {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("cannot send message: {e:?}");
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
                    log::error!("cannot send end-of-stream message: {e:?}");
                }
            }
        } else {
            log::warn!("unexpected: {message}");
        }
    }
    async fn stop(&self) {}
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
