use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::actor::State;
use crate::genes::Gene;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::NvError;
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
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub output: Option<Handle>,
    pub state: State<f64>,
    pub path: String,
    pub gene: Box<dyn Gene<f64> + Send + Sync>,
}

#[async_trait]
impl Actor for StateActor {
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        let Envelope {
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
                        match &message {
                            Message::EndOfStream {} => {
                                break;
                            }
                            _ => {
                                if self.update_state(message.clone()) {
                                    count += 1;
                                } else {
                                    log::trace!("{} init closing stream.", self.path);
                                    break;
                                }
                            }
                        }
                    }
                    log::trace!("{} init closing stream.", self.path);
                    stream_from.close();

                    log::debug!("{} finished init from {} events", self.path, count);

                    respond_or_log_error(respond_to, Ok(Message::EndOfStream {}));
                }
            }
            Message::Update { .. } => {
                log::trace!("{} handling update", self.path);

                if self.update_state(message.clone()) {
                    respond_or_log_error(respond_to, Ok(self.get_state_rpt()));
                } else {
                    log::error!("Error applying operators in ask");
                    respond_or_log_error(
                        respond_to,
                        Err(NvError {
                            reason: String::from("cannot apply operators"),
                        }),
                    );
                }
            }
            Message::Query { .. } => {
                // respond with a copy of our new state if this is an 'ask'
                respond_or_log_error(respond_to, Ok(self.get_state_rpt()));
            }
            m => {
                log::warn!("unexpected message: {m}");
            }
        }

        // report the update to our state to the output actor
        if let Some(output_handle) = &self.output {
            if let Err(err) = output_handle.tell(self.get_state_rpt()).await {
                log::error!("Error telling output actor: {err:?}");
            }
        }
    }
    async fn stop(&self) {}
}

/// actor private constructor
impl StateActor {
    fn update_state(&mut self, message: Message<f64>) -> bool {
        match self.gene.apply_operators(self.state.clone(), message) {
            Ok(new_state) => {
                self.state = new_state;
                true
            }
            Err(e) => {
                log::error!("Error applying operators in ask: {e:?}");
                false
            }
        }
    }

    fn get_state_rpt(&self) -> Message<f64> {
        Message::StateReport {
            path: self.path.clone(),
            values: self.state.clone(),
            datetime: OffsetDateTime::now_utc(), // TODO: should be from latest observations
                                                 // (maybe)
        }
    }

    /// state will populated from event store before any other processing via
    /// the lifecycle processing coordinated by the director
    fn new(
        path: String,
        receiver: mpsc::Receiver<Envelope<f64>>,
        output: Option<Handle>,
        gene: Box<dyn Gene<f64> + Send + Sync>,
    ) -> Self {
        let state = State::new();
        Self {
            receiver,
            output,
            state,
            path,
            gene,
        }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(
    path: String,
    bufsz: usize,
    gene: Box<dyn Gene<f64> + Send + Sync>,
    output: Option<Handle>,
) -> Handle {
    async fn start<'a>(mut actor: StateActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = StateActor::new(path, receiver, output, gene);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    actor_handle
}
