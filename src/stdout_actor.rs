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
    path: String,
}

#[async_trait]
impl Actor for StdoutActor {
    fn get_path(&mut self) -> String {
        self.path.clone()
    }

    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            path: _,
            message,
            respond_to_opt,
            datetime: _,
        } = envelope;
        match message {
            Message::PrintOneCmd { text } => println!("{}", text),
            Message::StateReport {
                path,
                datetime: _,
                values,
            } => println!("{} new state: {:?}", path, values),
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
        }
    }
}

/// actor private constructor
impl StdoutActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        StdoutActor {
            path: String::from("/"),
            receiver,
        }
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
