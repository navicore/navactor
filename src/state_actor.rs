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
impl<'a> Actor<'a> for StateActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            timestamp: _,
        } = envelope;

        match message {
            Message::UpdateCmd { path: _, values } => {
                self.state.extend(&values);
                // report the update to our state to the output actor
                if let Some(output_handle) = &self.output {
                    let logmsg = Message::PrintOneCmd {
                        text: format!("updating with: {:?}", &values),
                    };
                    output_handle.tell(logmsg).await;
                }
                // if this is a tell, respond with a copy of our new state
                if let Some(respond_to) = respond_to_opt {
                    let state_msg = Message::UpdateCmd {
                        path: String::from("/"),
                        values: self.state.clone(),
                    };
                    respond_to.send(state_msg).expect("can not reply to tell");
                }
            }
            Message::IsCompleteMsg {} => {
                log::debug!("complete");
                // we can only use the oneshot send once so priority is to
                // send it onward to an output if defined or if not, send to
                // the respond_to if defined
                if let Some(output_handle) = &self.output {
                    log::debug!("forwarding complete");
                    // report our new state to the output actor
                    let logmsg = Message::PrintOneCmd {
                        text: format!("new state: {:?}", self.state),
                    };
                    output_handle.tell(logmsg).await;
                    // forward the complete cmd to the output actor
                    let senv = MessageEnvelope {
                        message: message.clone(),
                        respond_to_opt,
                        ..Default::default()
                    };
                    output_handle.send(senv).await
                } else if let Some(respond_to) = respond_to_opt {
                    log::debug!("replying complete");
                    respond_to
                        .send(message)
                        .expect("can not reply to tell IsCompleteMsg");
                }
            }
            _ => {}
        }
    }
}

/// actor private constructor
impl<'a> StateActor {
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
    async fn start<'a>(mut actor: StateActor) {
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
