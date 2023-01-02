use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct StateActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

#[async_trait]
impl Actor for StateActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => match message {
                Message::UpdateCmd { values } => {
                    log::debug!("haha: {}", values.len());
                }
                Message::IsCompleteMsg {} => {
                    let senv = MessageEnvelope {
                        message,
                        respond_to_opt,
                    };
                    self.output.send(senv).await
                }
                _ => {}
            },
        }
    }
}

impl StateActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        StateActor { receiver, output }
    }
}

pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: StateActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StateActor::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
