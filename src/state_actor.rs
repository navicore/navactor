use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// the state actor is the heart of the system.  each digital twin has an
/// instance of actor keeping state computed from an arriving stream of
/// observations.
pub struct StateActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: Option<ActorHandle>,
    pub state: HashMap<i32, f64>,
    path: String,
}

#[async_trait]
impl Actor for StateActor {
    fn get_path(&mut self) -> String {
        self.path.clone()
    }

    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            timestamp: _,
        } = envelope;

        if let Message::UpdateCmd {
            timestamp: _,
            path: _,
            values,
        } = message
        {
            log::debug!("state actor {} update", self.path);

            self.state.extend(&values); //update state

            // report the update to our state to the output actor
            if let Some(output_handle) = &self.output {
                let logmsg = Message::PrintOneCmd {
                    text: format!("updating with: {:?}", &values),
                };
                output_handle.tell(logmsg).await;
            }

            // respond with a copy of our new state if this is an 'ask'
            if let Some(respond_to) = respond_to_opt {
                let state_msg = Message::UpdateCmd {
                    timestamp: Utc::now(),
                    path: self.path.clone(),
                    values: self.state.clone(),
                };
                respond_to.send(state_msg).expect("can not reply to ask");
            }
        }
    }
}

/// actor private constructor
impl StateActor {
    fn new(
        path: String,
        receiver: mpsc::Receiver<MessageEnvelope>,
        output: Option<ActorHandle>,
    ) -> Self {
        let state = HashMap::new(); // TODO: load from event store
        StateActor {
            path,
            receiver,
            output,
            state,
        }
    }
}

/// actor handle public constructor
pub fn new(path: String, bufsz: usize, output: Option<ActorHandle>) -> ActorHandle {
    async fn start<'a>(mut actor: StateActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StateActor::new(path, receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
