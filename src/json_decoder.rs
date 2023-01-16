use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::message::Observations;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

/// actor accepts numerical json and converts into the internal state data msg
pub struct JsonDecoder {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
    path: String,
}

fn extract_values_from_json(text: &str) -> Result<Observations, String> {
    let observations: Observations = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(observations)
}

fn extract_datetime(datetime_str: &str) -> DateTime<Utc> {
    match DateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S%z") {
        Ok(d) => d.with_timezone(&Utc),
        Err(e) => {
            log::warn!("can not parse datetime {} due to: {}", &datetime_str, e);
            Utc::now()
        }
    }
}

#[async_trait]
impl Actor for JsonDecoder {
    fn get_path(&mut self) -> String {
        self.path.clone()
    }
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            datetime,
        } = envelope;
        // match the messages we know how to decode and forward them and everything else to the
        // next hop
        match &message {
            Message::PrintOneCmd { text } => match extract_values_from_json(text) {
                Ok(observations) => {
                    //forward observations to actor
                    let msg = Message::UpdateCmd {
                        path: String::from(&observations.path),
                        datetime: extract_datetime(&observations.datetime),
                        values: observations.values,
                    };

                    // forward if output is configured
                    let senv = MessageEnvelope {
                        message: msg,
                        respond_to_opt, // delegate responding to an ask to director
                        datetime,
                    };
                    self.output.send(senv).await;
                }
                Err(error) => {
                    log::warn!("json parse error: {}", error);
                    if let Some(respond_to) = respond_to_opt {
                        let etxt = format!("json parse error: {}", error);
                        let emsg = Message::ErrorReport {
                            datetime: Utc::now(),
                            path: None,
                            text: etxt,
                        };
                        respond_to.send(emsg).expect("can not return error");
                    }
                }
            },
            m => {
                // forward everything else
                let senv = MessageEnvelope {
                    message: m.clone(),
                    respond_to_opt,
                    ..Default::default()
                };
                self.output.send(senv).await;
            }
        }
    }
}

/// actor private constructor
impl JsonDecoder {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        JsonDecoder {
            path: "/internal".to_string(),
            receiver,
            output,
        }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: JsonDecoder) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = JsonDecoder::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
