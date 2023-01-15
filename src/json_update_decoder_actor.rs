use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::message::Observations;
use crate::state_actor;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

/// actor accepts numerical json and converts into the internal state data msg
pub struct JsonUpdateDecoderActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: Option<ActorHandle>,
    pub actors: HashMap<String, ActorHandle>,
    path: String,
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
    fn get_path(&mut self) -> String {
        self.path.clone()
    }

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
                            path: observations.path.clone(),
                            values: observations.values,
                        };
                        let actor = self
                            .actors
                            .entry(observations.path.clone())
                            .or_insert(state_actor::new(observations.path.clone(), 8, None));

                        let response = actor.ask(msg).await;
                        log::debug!(
                            "update actor got single actor instance state {:?}",
                            response
                        );
                    }
                    Err(error) => {
                        log::warn!("{}", error); // TODO send back an error to respond_to
                    }
                },
                Message::IsCompleteMsg {} => {
                    log::debug!("complete");

                    // forward if we are configured with an output
                    if let Some(a) = &self.output {
                        let senv = MessageEnvelope {
                            message,
                            respond_to_opt,
                            ..Default::default()
                        };
                        a.send(senv).await // forward the good news
                    } else if let Some(respond_to) = respond_to_opt {
                        // else we're the end of the line so reply if this is an ask
                        respond_to.send(message).expect("can not reply to ask");
                    }
                }
                _ => {}
            },
        }
    }
}

/// actor private constructor
impl JsonUpdateDecoderActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: Option<ActorHandle>) -> Self {
        JsonUpdateDecoderActor {
            path: "/internal".to_string(),
            actors: HashMap::new(),
            receiver,
            output,
        }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: Option<ActorHandle>) -> ActorHandle {
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
