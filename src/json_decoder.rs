use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::ActorError;
use crate::message::ActorResult;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::MtHint;
use crate::message::Observations;
use crate::message::PathQuery;
use crate::nvtime::extract_datetime;
use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

/// actor accepts numerical json and converts into the internal state data msg
pub struct JsonDecoder {
    pub receiver: mpsc::Receiver<Envelope>,
    pub output: Handle,
}

fn extract_path_from_json(text: &str) -> Result<PathQuery, String> {
    let query: PathQuery = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(query)
}

fn extract_values_from_json(text: &str) -> Result<Observations, String> {
    let observations: Observations = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(observations)
}

#[async_trait]
impl Actor for JsonDecoder {
    async fn stop(&self) {}

    async fn handle_envelope(&mut self, envelope: Envelope) {
        let Envelope {
            message,
            respond_to,
            datetime,
            ..
        } = envelope;
        match message {
            Message::TextMsg {
                text,
                hint: MtHint::Query,
            } => self.handle_query_json(&text, respond_to, datetime).await,
            Message::TextMsg {
                text,
                hint: MtHint::Update,
            } => self.handle_update_json(&text, respond_to, datetime).await,
            m => {
                let senv = Envelope {
                    message: m,
                    respond_to,
                    ..Default::default()
                };
                self.send_or_log_error(senv).await;
            }
        }
    }
}

impl JsonDecoder {
    async fn handle_update_json(
        &self,
        text: &str,
        respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
        datetime: OffsetDateTime,
    ) {
        match extract_values_from_json(&text) {
            Ok(observations) => {
                log::trace!("json parsed");
                let msg = Message::Update {
                    path: observations.path,
                    datetime: extract_datetime(&observations.datetime),
                    values: observations.values,
                };

                let senv = Envelope {
                    message: msg,
                    respond_to,
                    datetime,
                    ..Default::default()
                };
                self.send_or_log_error(senv).await;
            }
            // TODO: refactor all parse errors to helper handler
            Err(error) => {
                log::warn!("json parse error: {}", error);
                if let Some(respond_to) = respond_to {
                    let etxt = format!("json parse error: {error}");
                    respond_to
                        .send(Err(ActorError { reason: etxt }))
                        .map_err(|e| {
                            log::error!("can not respond: {e:?}");
                        })
                        .ok();
                }
            }
        }
    }

    async fn handle_query_json(
        &self,
        text: &str,
        respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
        datetime: OffsetDateTime,
    ) {
        match extract_path_from_json(&text) {
            Ok(path_query) => {
                log::trace!("query json parsed");
                let msg = Message::Query {
                    path: path_query.path,
                };

                let senv = Envelope {
                    message: msg,
                    respond_to,
                    datetime,
                    ..Default::default()
                };
                self.send_or_log_error(senv).await;
            }
            // TODO: refactor all parse errors to helper handler
            Err(error) => {
                log::warn!("json parse error: {}", error);
                if let Some(respond_to) = respond_to {
                    let etxt = format!("json parse error: {error}");
                    respond_to
                        .send(Err(ActorError { reason: etxt }))
                        .map_err(|e| {
                            log::error!("can not respond: {e:?}");
                        })
                        .ok();
                }
            }
        }
    }

    async fn send_or_log_error(&self, value: Envelope)
    where
        Envelope: Send + std::fmt::Debug,
    {
        match self.output.send(value).await {
            Ok(_) => (),
            Err(e) => log::error!("cannot send: {:?}", e),
        }
    }
    /// actor private constructor
    const fn new(receiver: mpsc::Receiver<Envelope>, output: Handle) -> Self {
        Self { receiver, output }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(bufsz: usize, output: Handle) -> Handle {
    async fn start(mut actor: JsonDecoder) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = JsonDecoder::new(receiver, output);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    actor_handle
}
