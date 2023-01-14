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

// UNDER CONSTRUCTION
// UNDER CONSTRUCTION
// UNDER CONSTRUCTION
//
// need to extract timestamp and path as well as values from the new fmt json string

/// actor accepts numerical json and converts into the internal state data msg
pub struct JsonUpdateDecoderActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

fn extract_values_from_json(text: &String) -> Result<Observations, String> {
    let observations: Observations = match serde_json::from_str(text) {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };
    Ok(observations)
}

#[async_trait]
impl Actor for JsonUpdateDecoderActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
                timestamp: _,
            } => match &message {
                Message::PrintOneCmd { text } => match extract_values_from_json(text) {
                    Ok(observations) => {
                        let datetime: DateTime<Utc> = match DateTime::parse_from_str(
                            &observations.datetime,
                            "%Y-%m-%dT%H:%M:%S%z",
                            //"%Y-%m-%dT%H:%M:%S%.fZ",
                            //  "%Y-%m-%dT%H:%M:%S%.fZ",
                        ) {
                            Ok(d) => d.with_timezone(&Utc),
                            Err(e) => {
                                log::warn!(
                                    "can not parse datetime {} due to: {}",
                                    &observations.datetime,
                                    e
                                );
                                Utc::now()
                            }
                        };

                        let msg = Message::UpdateCmd {
                            timestamp: datetime,
                            path: observations.path,
                            values: observations.values,
                        };
                        self.output.tell(msg).await
                    }
                    Err(error) => {
                        log::warn!("{}", error); // TODO send back an error to respond_to
                    }
                },
                Message::IsCompleteMsg {} => {
                    let senv = MessageEnvelope {
                        message,
                        respond_to_opt,
                        ..Default::default()
                    };
                    log::debug!("complete");
                    self.output.send(senv).await // forward the good news
                }
                _ => {}
            },
        }
    }
}

/// actor private constructor
impl JsonUpdateDecoderActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        JsonUpdateDecoderActor { receiver, output }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: JsonUpdateDecoderActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = JsonUpdateDecoderActor::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
