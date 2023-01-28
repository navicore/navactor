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
    pub store_actor: Option<ActorHandle>,
    pub output: Option<ActorHandle>,
    pub actors: HashMap<String, ActorHandle>,
    namespace: String,
}

#[async_trait]
impl Actor for Director {
    async fn stop(&mut self) {}
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            ..
        } = envelope;

        match &message {
            Message::Update { path, .. } | Message::Query { path } => {
                // resurrect and forward if this is either Update or Query
                let response = self.update_actor(path, message.clone()).await;

                // the store actor to jrnl this Update for this path here.
                match message {
                    Message::Update { path: _, .. } => {
                        if let Some(store_actor) = &self.store_actor {
                            // waiting for confirm before
                            // triggering pipeline and risking
                            // process termination while store
                            // actor is still writing
                            let jrnl_msg = store_actor.ask(message).await;
                            if let Some(respond_to) = respond_to {
                                match jrnl_msg {
                                    Message::EndOfStream {} => {
                                        // reply with response if this is an 'ask' from the sender
                                        respond_to
                                            .send(response.clone())
                                            .expect("can not reply to ask");
                                    }
                                    Message::JrnlError { .. } => {
                                        respond_to.send(jrnl_msg).expect("can not reply to ask");
                                    }
                                    m => {
                                        log::warn!("Unexpected store message: {:?}", m);
                                    }
                                }
                            }
                        } else {
                            // TODO: fix this DRY violation - it is here because this code supports non-persist
                            // reply with response if this is an 'ask' from the sender
                            if let Some(respond_to) = respond_to {
                                respond_to
                                    .send(response.clone())
                                    .expect("can not reply to ask");
                            }
                        }
                    }
                    Message::Query { path: _, .. } => {
                        if let Some(respond_to) = respond_to {
                            respond_to
                                .send(response.clone())
                                .expect("can not reply to ask");
                        }
                    }
                    m => {
                        log::warn!("unexpected message: {:?}", m);
                    }
                }

                // forward response if output is configured
                if let Some(o) = &self.output {
                    let senv = MessageEnvelope {
                        message: response,
                        respond_to: None,
                        ..Default::default()
                    };
                    o.send(senv).await;
                }
            }
            Message::EndOfStream {} => {
                log::debug!("complete");

                if let Some(a) = &self.output {
                    let senv = MessageEnvelope {
                        message,
                        respond_to,
                        ..Default::default()
                    };
                    a.send(senv).await
                } else if let Some(respond_to) = respond_to {
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
        store_actor: Option<ActorHandle>,
    ) -> Self {
        Director {
            namespace,
            actors: HashMap::new(),
            receiver,
            output,
            store_actor,
        }
    }

    async fn update_actor(&mut self, path: &String, message: Message) -> Message {
        let mut actor_is_in_init = false;
        let actor = self.actors.entry(path.clone()).or_insert_with(|| {
            actor_is_in_init = true;
            state_actor::new(path.clone(), 8, None)
        });

        match &self.store_actor {
            Some(store_actor) if actor_is_in_init => {
                log::debug!(
                    "{} handling actor '{}' messsage w/ actor init via store",
                    self.namespace,
                    path
                );

                actor
                    .integrate(message.clone(), String::from(path), store_actor)
                    .await
            }
            _ => {
                log::debug!("{} handling actor '{}' messsage", self.namespace, path);
                actor.ask(message.clone()).await
            }
        }
    }
}

/// actor handle public constructor
pub fn new(
    namespace: String,
    bufsz: usize,
    output: Option<ActorHandle>,
    store_actor: Option<ActorHandle>,
) -> ActorHandle {
    async fn start(mut actor: Director) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = Director::new(namespace.clone(), receiver, output, store_actor);

    let actor_handle = ActorHandle::new(sender);

    tokio::spawn(start(actor));

    log::debug!("{} started", namespace);
    actor_handle
}
