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
impl Actor for JsonUpdateDecoderActor {
    fn get_path(&mut self) -> String {
        self.path.clone()
    }
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            datetime: _,
        } = envelope;
        match &message {
            Message::PrintOneCmd { text } => match extract_values_from_json(text) {
                Ok(observations) => {
                    //upsert actor
                    let actor = self
                        .actors
                        .entry(observations.path.clone())
                        .or_insert_with(|| state_actor::new(observations.path.clone(), 8, None));

                    //forward observations to actor
                    let msg = Message::UpdateCmd {
                        datetime: extract_datetime(&observations.datetime),
                        path: observations.path.clone(),
                        values: observations.values,
                    };
                    let response = actor.ask(msg).await;

                    // reply if this is an 'ask'
                    if let Some(respond_to) = respond_to_opt {
                        respond_to
                            .send(response.clone())
                            .expect("can not reply to ask");
                    }

                    // forward if output is configured
                    if let Some(o) = &self.output {
                        let senv = MessageEnvelope {
                            message: response.clone(),
                            respond_to_opt: None,
                            ..Default::default()
                        };
                        o.send(senv).await;
                    }
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
            m => log::warn!("unexpected message: {:?}", m),
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
