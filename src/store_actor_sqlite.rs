use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct StoreActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
}

#[async_trait]
impl Actor for StoreActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            datetime,
            stream_to: _,
            stream_from: _,
            next_message: _,
            next_message_respond_to: _,
        } = envelope;
        match message {
            Message::LoadCmd { path } => {

                // TODO: handle LoadCmd messages
                // 1. open db with the path as key
                // 2. write events to "stream_to"
                // 3. write next_message to "stream_to"
                // 4. write EndOfStream to "stream_to"
                // 5. close send???
            }
            m => log::warn!("unexpected: {:?}", m),
        }
    }
}

/// actor private constructor
impl StoreActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>) -> Self {
        StoreActor { receiver }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize) -> ActorHandle {
    async fn start(mut actor: StoreActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StoreActor::new(receiver);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
