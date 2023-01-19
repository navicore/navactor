use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::state_actor;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

// TODO:
// use rust std Path to update and persist petgraph graph Edges and
// lookup/upsert actor for each input record msg send

/// actor graph director creates a graph and instantiates all the actors that
/// it is forwarding commands to.  director also accepts metadata to create
/// and store graph edges to support arbitrary paths
pub struct Director {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub output: Option<ActorHandle>,
    pub actors: HashMap<String, ActorHandle>,
    namespace: String,
}

#[async_trait]
impl Actor for Director {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to_opt,
            datetime: _,
            stream_to: _,
            stream_from: _,
            next_message: _,
        } = envelope;

        match &message {
            Message::UpdateCmd {
                path,
                datetime: _,
                values: _,
            } => {
                log::debug!("{} handling actor messsage", self.namespace);
                //upsert actor
                let actor = self
                    .actors
                    .entry(path.clone())
                    .or_insert_with(|| state_actor::new(path.clone(), 8, None));

                let response = actor.ask(message).await;

                // reply with response if this is an 'ask' from the sender
                if let Some(respond_to) = respond_to_opt {
                    respond_to
                        .send(response.clone())
                        .expect("can not reply to ask");
                }

                // forward response if output is configured
                if let Some(o) = &self.output {
                    let senv = MessageEnvelope {
                        message: response.clone(),
                        respond_to_opt: None,
                        ..Default::default()
                    };
                    o.send(senv).await;
                }
            }
            Message::EndOfStream {} => {
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
impl Director {
    fn new(
        namespace: String,
        receiver: mpsc::Receiver<MessageEnvelope>,
        output: Option<ActorHandle>,
    ) -> Self {
        Director {
            namespace,
            actors: HashMap::new(),
            receiver,
            output,
        }
    }
}

/// actor handle public constructor
pub fn new(namespace: String, bufsz: usize, output: Option<ActorHandle>) -> ActorHandle {
    async fn start(mut actor: Director) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = Director::new(namespace, receiver, output);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
