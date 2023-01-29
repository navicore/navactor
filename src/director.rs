use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::actor::ActorResult;
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
                match self.update_actor(path, message.clone()).await {
                    Ok(response) => {
                        log::trace!("got  response {response:?}");

                        // the store actor to jrnl this Update for this path here.
                        match message {
                            Message::Update { path: _, .. } => {
                                if let Some(store_actor) = &self.store_actor {
                                    let jrnl_msg = store_actor.ask(message).await;
                                    if let Some(respond_to) = respond_to {
                                        match jrnl_msg {
                                            Ok(r) => match r {
                                                Message::EndOfStream {} => {
                                                    respond_to
                                                        .send(response.clone())
                                                        .expect("can not reply to ask");
                                                }
                                                Message::JrnlError { .. } => {
                                                    respond_to
                                                        .send(r)
                                                        .expect("can not reply to ask");
                                                }
                                                m => {
                                                    log::warn!("Unexpected store message: {:?}", m);
                                                }
                                            },
                                            Err(e) => {
                                                log::warn!("error {e}");
                                            }
                                        }
                                    }
                                } else {
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
                            o.send(senv).await.expect("receiver not ready");
                        }
                    }
                    Err(e) => {
                        log::warn!("error {e}");
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

    async fn update_actor(&mut self, path: &String, message: Message) -> ActorResult<Message> {
        let mut actor_is_in_init = false;
        let actor = self.actors.entry(path.clone()).or_insert_with(|| {
            actor_is_in_init = true;
            state_actor::new(path.clone(), 8, None)
        });

        // TODO: do another ask here only if integrate returns success or if this is not an init
        match &self.store_actor {
            Some(store_actor) if actor_is_in_init => {
                log::debug!(
                    "{} handling actor '{}' messsage w/ actor init via store",
                    self.namespace,
                    path
                );

                let result = actor.integrate(String::from(path), store_actor).await;
                if let Ok(Message::EndOfStream {}) = result {
                    log::debug!(
                        //TODO:
                        "ejs *********** this should be a result that lets us proceed with an ask"
                    );
                    actor.ask(message.clone()).await
                } else {
                    result // TODO: stop this ... should be results
                }
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
