use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use std::collections::HashMap;
use time::OffsetDateTime;
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
    async fn stop(&mut self) {}
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            stream_from,
            next_message,
            next_message_respond_to,
            ..
        } = envelope;

        if let Message::InitCmd {} = message {
            // this is an init so read your old events to recalculate your state
            if let Some(mut stream_from) = stream_from {
                log::debug!("{} init", self.path);
                let mut count = 0;

                while let Some(message) = stream_from.recv().await {
                    if !self.update_state(message.clone()) {
                        stream_from.close();
                    } else {
                        count += 1;
                    }
                }

                log::debug!("{} finished init from {} events", self.path, count);

                // once old state is recalculated, you can now process the new event
                if let Some(nm) = next_message {
                    self.update_state(nm);

                    // respond with a copy of our new state if this is an 'ask'
                    if let Some(r) = next_message_respond_to {
                        r.send(self.get_state_rpt()).expect("can not reply to ask");
                    }
                }
            }
        } else {
            self.update_state(message.clone());
        }

        // report the update to our state to the output actor
        if let Some(output_handle) = &self.output {
            output_handle.tell(self.get_state_rpt()).await;
        }

        // respond with a copy of our new state if this is an 'ask'
        if let Some(respond_to) = respond_to {
            respond_to
                .send(self.get_state_rpt())
                .expect("can not reply to ask");
        }
    }
}

/// actor private constructor
impl StateActor {
    fn update_state(&mut self, message: Message) -> bool {
        let mut updated = false;
        match message {
            Message::Update { values, .. } => {
                updated = true;

                self.state.extend(&values); //update state
            }
            Message::Query { path: _ } => {
                // no impl because all "respond_to" requests get a state_rpt
            }
            Message::EndOfStream {} => {
                log::trace!("{} end of update stream", self.path);
            }
            m => {
                log::warn!("unexpected message in update stream: {:?}", m);
            }
        }
        updated
    }

    fn get_state_rpt(&self) -> Message {
        Message::StateReport {
            path: self.path.clone(),
            values: self.state.clone(),
            datetime: OffsetDateTime::now_utc(),
        }
    }

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
