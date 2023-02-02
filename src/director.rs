use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::ActorError;
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
    async fn stop(&self) {}

    // TODO: I've spent a lot of time trying to refactor this into 3 functions
    // but I'm stuck on them all dragging mut self along... Learning....
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        log::trace!(
            "director namespace {} handling_envelope {envelope:?}",
            self.namespace
        );
        let MessageEnvelope {
            message,
            respond_to,
            ..
        } = envelope;

        match &message {
            Message::Update { path, .. } | Message::Query { path } => {
                // resurrect and forward if this is either Update or Query
                let mut actor_is_in_init = false;
                let actor = self.actors.entry(path.clone()).or_insert_with(|| {
                    actor_is_in_init = true;
                    state_actor::new(path.clone(), 8, None)
                });

                let loaded: bool = match &self.store_actor {
                    Some(store_actor) if actor_is_in_init => {
                        match actor.integrate(String::from(path), store_actor).await {
                            Ok(_) => {
                                // actor has read its journal
                                true
                            }
                            Err(e) => {
                                log::error!("can not load actor: {e}");
                                false
                            }
                        }
                    }
                    _ => true,
                };

                if loaded {
                    let journaled: bool = match message.clone() {
                        Message::Update { path: _, .. } => {
                            if let Some(store_actor) = &self.store_actor {
                                // jrnl the new msg
                                let jrnl_msg = store_actor.ask(message.clone()).await;
                                match jrnl_msg {
                                    Ok(r) => match r {
                                        Message::EndOfStream {} => {
                                            // successfully jrnled the msg, it is now safe to
                                            // send it to the actor to process
                                            true
                                        }
                                        m => {
                                            log::warn!("Unexpected store message: {m:?}");
                                            false
                                        }
                                    },
                                    Err(e) => {
                                        log::warn!("error {e}");
                                        false
                                    }
                                }
                            } else {
                                // jrnl is disabled, jsut process the message
                                true
                            }
                        }
                        Message::Query { path: _, .. } => true,
                        m => {
                            log::warn!("unexpected message: {:?}", m);
                            false
                        }
                    };

                    if journaled {
                        //send message to the actor and support ask results
                        let r = actor.ask(message.clone()).await;
                        if let Some(respond_to) = respond_to {
                            respond_to.send(r.clone()).expect("can not reply to ask");
                        }
                        // forward response if output is configured
                        if let Some(o) = &self.output {
                            if let Ok(message) = r {
                                let senv = MessageEnvelope {
                                    message,
                                    respond_to: None,
                                    ..Default::default()
                                };
                                o.send(senv).await.expect("receiver not ready");
                            }
                        }
                    } else {
                        log::error!("cannot journal input to actor {path} - see logs");
                        if let Some(respond_to) = respond_to {
                            respond_to
                                .send(Err(ActorError {
                                    reason: format!("cannot journal input to actor {path}"),
                                }))
                                .expect("can not reply to ask");
                        }
                    }
                } else {
                    log::error!("cannot load actor {path} - see logs");
                    if let Some(respond_to) = respond_to {
                        respond_to
                            .send(Err(ActorError {
                                reason: format!("cannot load actor {path}"),
                            }))
                            .expect("can not reply to ask");
                    }
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
                    a.send(senv).await.expect("cannot send");
                } else if let Some(respond_to) = respond_to {
                    // else we're the end of the line so reply if this is an ask
                    respond_to.send(Ok(message)).expect("can not reply to ask");
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
        Self {
            namespace,
            actors: HashMap::new(),
            receiver,
            output,
            store_actor,
        }
    }
}

/// actor handle public constructor
#[must_use] pub fn new(
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
