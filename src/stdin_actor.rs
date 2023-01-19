use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::io::stdin;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::mpsc;

/// the stdin actor is only used in CLI mode.  it gets a single command to
/// read from stdin and it reads until the EOF.  once it sees EOF, it sends
/// a EndOfStream msg to the next hop to trigger any cleanup and shutdown.
pub struct StdinActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

#[async_trait]
impl Actor for StdinActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            datetime: _,
            stream_to: _,
            stream_from: _,
            next_message: _,
        } = envelope;

        if let Message::ReadAllCmd {} = message {
            let mut lines = BufReader::new(stdin()).lines();

            while let Some(text) = lines.next_line().await.expect("failed to read stream") {
                let msg = Message::PrintOneCmd { text };
                self.output.tell(msg).await
            }

            // forward the respond_to handle so that the output actor can respond when all
            // is printed
            let complete_msg = Message::EndOfStream {};
            let senv = MessageEnvelope {
                message: complete_msg,
                respond_to_opt,
                ..Default::default()
            };
            self.output.send(senv).await
        } else {
            log::warn!("unexpected: {:?}", message);
        }
    }
}

/// actor private constructor
impl StdinActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        StdinActor { receiver, output }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: StdinActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StdinActor::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
