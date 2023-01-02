use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

pub struct JsonStateActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

#[async_trait]
impl Actor for JsonStateActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => match message {
                Message::PrintOneCmd { text } => {
                    let values: serde_json::Value = match serde_json::from_str(text.as_str()) {
                        Ok(values) => values,
                        Err(e) => {
                            log::warn!("{}", e);
                            return;
                        }
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
                    }
                    let msg = Message::UpdateCmd { values: map };
                    self.output.tell(msg).await
                }
                Message::IsCompleteMsg {} => {
                    let senv = MessageEnvelope {
                        message,
                        respond_to_opt,
                    };
                    self.output.send(senv).await
                }
                _ => {}
            },
        }
    }
}

impl JsonStateActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        JsonStateActor { receiver, output }
    }
}

pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: JsonStateActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = JsonStateActor::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
