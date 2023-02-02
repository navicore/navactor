use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::actor::ActorState;
use crate::genes::DefaultGene;
use crate::genes::Gene;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::mpsc;

// TODO: reject late reporters based on genes
// TODO: reject late reporters based on genes
// TODO: reject late reporters based on genes
// TODO: reject late reporters based on genes
// TODO: by default, counters are ok for late reports but guages are not

/// the state actor is the heart of the system.  each digital twin has an
/// instance of actor keeping state computed from an arriving stream of
/// observations.
pub struct StateActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: Option<ActorHandle>,
    pub state: ActorState<f64>,
    pub path: String,
    pub gene: Box<dyn Gene + Send + Sync>,
}

#[async_trait]
impl Actor for StateActor {
    async fn stop(&self) {}
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            stream_from,
            ..
        } = envelope;

        match message {
            Message::InitCmd { .. } => {
                log::trace!("{} init started...", self.path);
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

                    if let Some(respond_to) = respond_to {
                        respond_to
                            .send(Ok(Message::EndOfStream {}))
                            .expect("can not reply to init");
                    }
                }
            }
            Message::Update { .. } => {
                log::trace!("{} handling update", self.path);
                self.update_state(message.clone());
                if let Some(respond_to) = respond_to {
                    respond_to
                        .send(Ok(self.get_state_rpt()))
                        .expect("can not reply to ask");
                }
            }
            Message::Query { .. } => {
                // respond with a copy of our new state if this is an 'ask'
                if let Some(respond_to) = respond_to {
                    respond_to
                        .send(Ok(self.get_state_rpt()))
                        .expect("can not reply to ask");
                }
            }
            m => {
                log::warn!("unexpected message: {m:?}");
            }
        }

        // report the update to our state to the output actor
        if let Some(output_handle) = &self.output {
            output_handle
                .tell(self.get_state_rpt())
                .await
                .expect("cannot tell");
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

    /// state will populated from event store before any other processing via
    /// the lifecycle processing coordinated by the director
    fn new(
        path: String,
        receiver: mpsc::Receiver<MessageEnvelope>,
        output: Option<ActorHandle>,
        //gene: &Gene,
    ) -> Self {
        let gene = Box::new(DefaultGene::new());
        let state = ActorState::new();
        Self { receiver, output, state, path, gene }
    }
}

/// actor handle public constructor
#[must_use] pub fn new(path: String, bufsz: usize, output: Option<ActorHandle>) -> ActorHandle {
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
