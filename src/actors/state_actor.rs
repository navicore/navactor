//! The state actor is the core component of the system. Each digital twin has
//! an instance of the actor keeping state computed from an arriving stream of
//! observations. It processes three types of messages: `InitCmd`, `Update`, and
//! `Query`. The `InitCmd` message initializes the state of the actor from
//! previous events. The `Update` message updates the state of the actor and
//! responds with the current state report. The `Query` message simply responds
//! with a copy of the current state report. The state actor also reports the
//! update to the state to the output actor if it is specified.

use crate::actors::actor::respond_or_log_error;
use crate::actors::actor::Actor;
use crate::actors::actor::Handle;
use crate::actors::actor::State;
use crate::actors::genes::gene::Gene;
use crate::actors::message::Envelope;
use crate::actors::message::Message;
use crate::actors::message::NvError;
use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

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
                trace!("{} init started...", self.path);
                // this is an init so read your old events to recalculate your state
                if let Some(mut stream_from) = stream_from {
                    debug!("{} init", self.path);
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
                                    trace!("{} init closing stream.", self.path);
                                    break;
                                }
                            }
                        }
                    }
                    trace!("{} init closing stream.", self.path);
                    stream_from.close();

                    debug!("{} finished init from {} events", self.path, count);

                    respond_or_log_error(respond_to, Ok(Message::EndOfStream {}));
                }
            }
            Message::Observations { .. } => {
                trace!("{} handling update", self.path);

                if self.update_state(message.clone()) {
                    respond_or_log_error(respond_to, Ok(self.get_state_rpt()));
                } else {
                    error!("Error applying operators in ask");
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
                warn!("unexpected message: {m}");
            }
        }

        // report the update to our state to the output actor
        if let Some(output_handle) = &self.output {
            if let Err(err) = output_handle.tell(self.get_state_rpt()).await {
                error!("Error telling output actor: {err:?}");
            }
        }
    }
    async fn stop(&self) {}

    async fn start(&mut self) {}
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
                error!("Error applying operators in ask: {e:?}");
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
