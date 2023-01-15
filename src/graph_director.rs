use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::state_actor;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

// TODO:
//
//
// use rust std Path to update petgraph graph Edges and lookup/upsert actor for
// each input record msg send

/// actor graph actor creates a graph and instantiates all the actors that
/// it is messaging commands to.  it also accepts metadata to create edge
/// objects in the graph
pub struct GraphDirector {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: ActorHandle,
    pub actors: HashMap<String, ActorHandle>,
    namespace: String,
}

#[async_trait]
impl Actor for GraphDirector {
    fn get_path(&mut self) -> String {
        self.namespace.clone()
    }

    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            datetime,
        } = envelope;
        match &message {
            Message::UpdateCmd {
                datetime: _,
                path,
                values: _,
            } => {
                //upsert actor
                let actor = self
                    .actors
                    .entry(path.clone())
                    .or_insert_with(|| state_actor::new(path.clone(), 8, None));

                let response = actor.ask(message).await;

                // forward
                let senv = MessageEnvelope {
                    message: response.clone(),
                    respond_to_opt: None,
                    ..Default::default()
                };
                self.output.send(senv).await;
            }
            Message::IsCompleteMsg {} => {
                let senv = MessageEnvelope {
                    message,
                    respond_to_opt,
                    datetime,
                };

                // forward
                self.output.send(senv).await // forward the good news
            }
            m => log::warn!("unexpected message: {:?}", m),
        }
    }
}

/// actor private constructor
impl GraphDirector {
    fn new(
        namespace: String,
        receiver: mpsc::Receiver<MessageEnvelope>,
        output: ActorHandle,
    ) -> Self {
        GraphDirector {
            namespace,
            actors: HashMap::new(),
            receiver,
            output,
        }
    }
}

/// actor handle public constructor
pub fn new(namespace: String, bufsz: usize, output: ActorHandle) -> ActorHandle {
    async fn start(mut actor: GraphDirector) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = GraphDirector::new(namespace, receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
