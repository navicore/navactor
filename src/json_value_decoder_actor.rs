use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

// FOR TESTING - DEPRECATED
// this just reads a jsonl file of values w/o any paths or timestamps

/// actor accepts numerical json and converts into the internal state data msg
pub struct JsonValueDecoderActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

fn extract_values_from_json(text: &str) -> Result<HashMap<i32, f64>, String> {
    let values: serde_json::Value = match serde_json::from_str(text) {
        Ok(values) => values,
        Err(e) => return Err(e.to_string()),
    };
    let mut map: HashMap<i32, f64> = HashMap::new();
    if let Some(obj) = values.as_object() {
        for (key, value) in obj.iter() {
            if let Ok(key) = key.parse::<i32>() {
                if let Some(value) = value.as_f64() {
                    map.insert(key, value);
                } else {
                    log::warn!("not numeric value: {}", value)
                    // TODO: return error if respond_to is available
                }
            } else {
                log::warn!("not numeric key: {}", key)
                // TODO: return error if respond_to is available
            }
        }
        Ok(map)
    } else {
        Err("invalid json".to_string())
    }
}

#[async_trait]
impl Actor for JsonValueDecoderActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            timestamp: _,
        } = envelope;
        match &message {
            Message::PrintOneCmd { text } => {
                match extract_values_from_json(text) {
                    Ok(values) => {
                        let path = String::from("/");
                        let msg = Message::UpdateCmd {
                            timestamp: Utc::now(),
                            path,
                            values,
                        };
                        self.output.tell(msg).await
                    }
                    Err(error) => {
                        log::warn!("{}", error); // TODO send back an error to respond_to
                    }
                }
            }
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
        }
    }
}

/// actor private constructor
impl JsonValueDecoderActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        JsonValueDecoderActor { receiver, output }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: JsonValueDecoderActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = JsonValueDecoderActor::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
