use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::genes::GuageAndAccumGene;
use crate::message::ActorError;
use crate::message::ActorResult;
use crate::message::Envelope;
use crate::message::Message;
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
    pub receiver: mpsc::Receiver<Envelope>,
    pub store_actor: Option<Handle>,
    pub output: Option<Handle>,
    pub actors: HashMap<String, Handle>,
    namespace: String,
}

// fn respond_or_log_error(
//     respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
//     result: ActorResult<Message>,
// ) {
//     {
//         if let Some(respond_to) = respond_to {
//             match respond_to.send(result) {
//                 Ok(_) => (),
//                 Err(err) => {
//                     log::error!("Cannot respond to 'ask' with confirmation: {:?}", err);
//                 }
//             }
//         }
//     }
// }
//
#[async_trait]
impl Actor for Director {
    async fn stop(&self) {}

    #[allow(clippy::too_many_lines)]
    async fn handle_envelope(&mut self, envelope: Envelope) {
        log::trace!(
            "director namespace {} handling_envelope {envelope:?}",
            self.namespace
        );
        let Envelope {
            message,
            respond_to,
            ..
        } = envelope;

        match &message {
            Message::Update { .. } => self.handle_update_or_query(message, respond_to).await,
            Message::Query { .. } => self.handle_update_or_query(message, respond_to).await,
            Message::EndOfStream {} => self.handle_end_of_stream(message, respond_to).await,
            m => log::warn!("unexpected message: {:?}", m),
        }
    }
}

/// actor private constructor
impl Director {
    async fn handle_end_of_stream(
        &self,
        message: Message,
        respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
    ) {
        log::debug!("complete");

        if let Some(a) = &self.output {
            let senv = Envelope {
                message,
                respond_to,
                ..Default::default()
            };
            a.send(senv)
                .await
                .map_err(|e| {
                    log::error!("cannot send: {e:?}");
                })
                .ok();
        } else {
            respond_or_log_error(respond_to, Ok(message.clone()));
        }
    }

    async fn handle_update_or_query(
        &mut self,
        message: Message,
        respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
    ) {
        if let Message::Update { path, .. } | Message::Query { path } = &message {
            // resurrect and forward if this is either Update or Query
            let mut actor_is_in_init = false;
            let actor = self.actors.entry(path.clone()).or_insert_with(|| {
                actor_is_in_init = true;
                // TODO: look up the gene by path
                let gene = Box::new(GuageAndAccumGene {
                    ..Default::default()
                });
                state_actor::new(path.clone(), 8, gene, None)
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
                    respond_or_log_error(respond_to, r.clone());
                    // TODO: extract to actor lib
                    if let Some(o) = &self.output {
                        if let Ok(message) = r {
                            let senv = Envelope {
                                message,
                                respond_to: None,
                                ..Default::default()
                            };
                            match o.send(senv).await {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!("can not forward: {e:?}");
                                }
                            }
                        }
                    }
                } else {
                    log::error!("cannot journal input to actor {path} - see logs");
                    respond_or_log_error(
                        respond_to,
                        Err(ActorError {
                            reason: format!("cannot journal input to actor {path}"),
                        }),
                    );
                }
            } else {
                log::error!("cannot load actor {path} - see logs");
                respond_or_log_error(
                    respond_to,
                    Err(ActorError {
                        reason: format!("cannot load actor {path}"),
                    }),
                );
            }
        }
    }
    fn new(
        namespace: String,
        receiver: mpsc::Receiver<Envelope>,
        output: Option<Handle>,
        store_actor: Option<Handle>,
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
#[must_use]
pub fn new(
    namespace: &String,
    bufsz: usize,
    output: Option<Handle>,
    store_actor: Option<Handle>,
) -> Handle {
    async fn start(mut actor: Director) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = Director::new(namespace.clone(), receiver, output, store_actor);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    log::debug!("{} started", namespace);
    actor_handle
}
