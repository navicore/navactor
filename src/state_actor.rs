use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// the state actor is the core of the system.  each digital twin has an
/// instance of actor keeping state calculated from the stream of observations
/// sent to the system.
pub struct StateActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: Option<ActorHandle>,
    pub state: HashMap<i32, f64>,
}

#[async_trait]
impl Actor for StateActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
        } = envelope;

        match message {
            Message::UpdateCmd { values } => {
                self.state.extend(&values);
                if let Some(output_handle) = &self.output {
                    let logmsg = Message::PrintOneCmd {
                        text: format!("updating with: {:?}", &values),
                    };
                    output_handle.tell(logmsg).await;
                }
            }
            Message::IsCompleteMsg {} => {
                if let Some(output_handle) = &self.output {
                    let logmsg = Message::PrintOneCmd {
                        text: format!("new state: {:?}", self.state),
                    };
                    output_handle.tell(logmsg).await;
                    let senv = MessageEnvelope {
                        message,
                        respond_to_opt,
                    };
                    output_handle.send(senv).await
                }
            }
            _ => {}
        }
    }
}

/// actor private constructor
impl StateActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: Option<ActorHandle>) -> Self {
        let state = HashMap::new(); // TODO: load from event store
        StateActor {
            receiver,
            output,
            state,
        }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: Option<ActorHandle>) -> ActorHandle {
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
