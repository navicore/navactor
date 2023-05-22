//! only used to support the cli ifc and stdin_actor
use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::Envelope;
use crate::message::GeneMapping;
use crate::message::Message;
use crate::message::MtHint;
use crate::message::NvError;
use crate::message::NvResult;
use crate::message::Observations;
use crate::message::PathQuery;
use crate::nvtime::extract_datetime;
use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;
use tracing::debug;
use tracing::error;
use tracing::trace;

pub struct JsonDecoder {
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub output: Handle,
}

fn extract_path_from_json(text: &str) -> Result<PathQuery, String> {
    let query: PathQuery = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(query)
}

fn extract_gene_mapping_from_json(text: &str) -> Result<GeneMapping, String> {
    let gene_mapping: GeneMapping = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(gene_mapping)
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
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        let Envelope {
            message,
            respond_to,
            datetime,
            ..
        } = envelope;
        match message {
            Message::Content {
                text,
                hint: MtHint::Query,
                path: _,
            } => self.handle_query_json(&text, respond_to, datetime).await,
            Message::Content {
                text,
                hint: MtHint::Update,
                path: _,
            } => self.handle_update_json(&text, respond_to, datetime).await,
            Message::Content {
                text: _,
                hint: MtHint::GeneMappingQuery,
                path,
            } => {
                self.handle_gene_mapping_query(path, respond_to, datetime)
                    .await;
            }
            Message::Content {
                text,
                hint: MtHint::GeneMapping,
                path: _,
            } => {
                self.handle_gene_mapping_json(&text, respond_to, datetime)
                    .await;
            }
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

    async fn stop(&self) {}

    async fn start(&mut self) {}
}

impl JsonDecoder {
    async fn handle_gene_mapping_json(
        &self,
        json_str: &str,
        respond_to: Option<tokio::sync::oneshot::Sender<NvResult<Message<f64>>>>,
        datetime: OffsetDateTime,
    ) {
        debug!("processing mapping update");
        match extract_gene_mapping_from_json(json_str) {
            Ok(gene_mapping) => {
                let msg = Message::GeneMapping {
                    path: gene_mapping.path,
                    gene_type: gene_mapping.gene_type,
                };

                let senv = Envelope {
                    message: msg,
                    respond_to,
                    datetime,
                    ..Default::default()
                };
                self.send_or_log_error(senv).await;
            }
            Err(error) => {
                error!("error processing mapping update: {error}");
                respond_or_log_error(
                    respond_to,
                    Err(NvError {
                        reason: format!("json parse error: {error:?}"),
                    }),
                );
            }
        }
    }

    async fn handle_gene_mapping_query(
        &self,
        path: Option<String>,
        respond_to: Option<tokio::sync::oneshot::Sender<NvResult<Message<f64>>>>,
        datetime: OffsetDateTime,
    ) {
        debug!("processing gene mapping query");
        let msg = Message::Content {
            path,
            text: String::new(),
            hint: MtHint::GeneMappingQuery,
        };

        let senv = Envelope {
            message: msg,
            respond_to,
            datetime,
            ..Default::default()
        };
        self.send_or_log_error(senv).await;
    }

    async fn handle_update_json(
        &self,
        json_str: &str,
        respond_to: Option<tokio::sync::oneshot::Sender<NvResult<Message<f64>>>>,
        datetime: OffsetDateTime,
    ) {
        match extract_values_from_json(json_str) {
            Ok(observations) => {
                trace!("json parsed");
                match extract_datetime(&observations.datetime) {
                    Ok(dt) => {
                        let msg = Message::Update {
                            path: observations.path,
                            datetime: dt,
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
                    Err(e) => {
                        error!("cannot parse datetime: {e}");
                    }
                }
            }
            Err(error) => {
                respond_or_log_error(
                    respond_to,
                    Err(NvError {
                        reason: format!("json parse error: {error:?}"),
                    }),
                );
            }
        }
    }

    async fn handle_query_json(
        &self,
        json_str: &str,
        respond_to: Option<tokio::sync::oneshot::Sender<NvResult<Message<f64>>>>,
        datetime: OffsetDateTime,
    ) {
        match extract_path_from_json(json_str) {
            Ok(path_query) => {
                trace!("query json parsed");
                let msg = Message::Query {
                    path: path_query.path,
                    hint: MtHint::State,
                };

                let senv = Envelope {
                    message: msg,
                    respond_to,
                    datetime,
                    ..Default::default()
                };
                self.send_or_log_error(senv).await;
            }
            Err(error) => {
                respond_or_log_error(
                    respond_to,
                    Err(NvError {
                        reason: format!("json parse error: {error:?}"),
                    }),
                );
            }
        }
    }

    async fn send_or_log_error(&self, envelope: Envelope<f64>)
    where
        Envelope<f64>: Send + std::fmt::Debug,
    {
        match self.output.send(envelope).await {
            Ok(_) => (),
            Err(e) => error!("cannot send: {:?}", e),
        }
    }

    /// actor private constructor
    const fn new(receiver: mpsc::Receiver<Envelope<f64>>, output: Handle) -> Self {
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
