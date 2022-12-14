use std::collections::HashMap;
use tokio::sync::mpsc;
extern crate serde;
extern crate serde_json;

// TODO:
//
// state actor graph actor creates a graph and instantiates all the actors that
// it is messaging commands to
//
// hold off on all extractor work - might be a data-wrangling until that should
// not pollute this impl.
//
// new json supports (1) observations, (2) metadata used to update graph
//
// parse new actor update json that has timestamp, path, observations
//
// update envelope to have nv timestamp and msgtype (1-data "update or delete",
// or 2-meta for soft links)
//
// use rust std Path to update petgraph graph Edges and lookup/upsert actor for
// each input record msg send

/// actor accepts numerical json and converts into the internal state data msg
pub struct GraphDirector {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
}

fn extract_values_from_json(text: &String) -> Result<HashMap<i32, f64>, String> {
    let values: serde_json::Value = match serde_json::from_str(text.as_str()) {
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
impl Actor for GraphDirector {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope {
                message,
                respond_to_opt,
            } => match &message {
                Message::PrintOneCmd { text } => match extract_values_from_json(text) {
                    Ok(values) => {
                        let msg = Message::UpdateCmd { values };
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
impl GraphDirector {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, output: ActorHandle) -> Self {
        GraphDirector { receiver, output }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: GraphDirector) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = GraphDirector::new(receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
