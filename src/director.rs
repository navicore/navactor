//!The `Director` struct is a graph director that creates a graph and instantiates all the actors
//!that it is forwarding commands to. The director also accepts metadata to create and store graph
//!edges to support arbitrary paths.
//!
//!The ``Director`` is responsible for handling `Envelope<f64>` messages received from its
//!associated `mpsc::Receiver`. The `Director` instantiates and forwards the received message to
//!the appropriate actor specified in the message's path field. If the actor doesn't exist, the
//!`Director` creates a new one by looking up the corresponding gene.
//!
//!The `Director` has several private functions that support its main functionality. These
//!functions are responsible for handling messages of different types (`Update`, `Query`,
//!`EndOfStream`), integrating newly created actors, journaling messages, and forwarding actor
//!results to an optional output. The `Director` also has a public constructor function that
//!creates a new handle for the `Director` actor, which is used to send messages to the `Director`.
//!
//!The `Director` is also responsible for creating and storing graph edges to support arbitrary
//!paths.
//!
//!The `Director` uses other Rust crates and libraries, such as `tokio`, `async_trait`,
//!`std::collections::HashMap`, and others.

use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::genes::GuageAndAccumGene;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::NvError;
use crate::message::NvResult;
use crate::state_actor;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::oneshot::Sender;

// TODO:
// use rust std Path to update and persist petgraph graph Edges and
// lookup/upsert actor for each input record msg send

/// This struct represents a graph director that creates a graph and instantiates all the actors
/// that it is forwarding commands to. The director also accepts metadata to create and store graph
/// edges to support arbitrary paths.
pub struct Director {
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub store_actor: Option<Handle>,
    pub output: Option<Handle>,
    pub actors: HashMap<String, Handle>,
    namespace: String,
}

#[async_trait]
impl Actor for Director {
    // This function is called when an envelope is received by the Director actor
    #[allow(clippy::too_many_lines)]
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        log::trace!(
            "director namespace {} handling_envelope {envelope}",
            self.namespace
        );
        let Envelope {
            message,
            respond_to,
            ..
        } = envelope;

        match &message {
            // If the message is an update or a query, handle it by calling the corresponding function
            Message::Update { .. } => self.handle_update_or_query(message, respond_to).await,
            Message::Query { .. } => self.handle_update_or_query(message, respond_to).await,
            // If the message is an EndOfStream message, handle it by forwarding it to the output actor
            // or by sending the response directly back to the original requester
            Message::EndOfStream {} => self.handle_end_of_stream(message, respond_to).await,
            // If the message is unexpected, log an error and respond with an NvError
            m => {
                let emsg = format!("unexpected message: {m}");
                log::error!("{emsg}");
                respond_or_log_error(respond_to, Err(NvError { reason: emsg }));
            }
        }
    }

    async fn stop(&self) {}
}

/// This function returns true once the newly resurrected actor reads all its journal.
async fn journal_message(message: Message<f64>, store_actor: &Option<Handle>) -> bool {
    if let Some(store_actor) = store_actor {
        // jrnl the new msg
        let jrnl_msg = store_actor.ask(message.clone()).await;
        match jrnl_msg {
            Ok(r) => match r {
                // If the message was successfully journalled, it is safe to send it to the actor to process
                Message::EndOfStream {} => {
                    // successfully jrnled the msg, it is now safe to
                    // send it to the actor to process
                    true
                }
                // If the store message is unexpected, log a warning and return false
                m => {
                    log::warn!("Unexpected store message: {m}");
                    false
                }
            },
            Err(e) => {
                log::warn!("error {e}");
                false
            }
        }
    } else {
        // If journaling is disabled, just process the message and return true
        true
    }
}

async fn forward_actor_result(result: NvResult<Message<f64>>, output: &Option<Handle>) {
    //forward to optional output
    if let Some(o) = output {
        if let Ok(message) = result {
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
}

async fn handle_post_jrnl_procesing(
    journaled: bool,
    message: Message<f64>,
    respond_to: Option<Sender<NvResult<Message<f64>>>>,
    actor: &Handle,
    output: &Option<Handle>,
) {
    if journaled {
        //send message to the actor and support ask results
        let r = actor.ask(message).await;
        respond_or_log_error(respond_to, r.clone());

        //forward to optional output
        forward_actor_result(r, output).await;
    } else {
        log::error!("cannot journal input to actor - see logs");
        respond_or_log_error(
            respond_to,
            Err(NvError {
                reason: String::from("cannot journal input to actor"),
            }),
        );
    }
}

/// actor private constructor
impl Director {
    async fn handle_end_of_stream(
        &self,
        message: Message<f64>,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        log::debug!("complete");

        // forward message to output but direct response directly back to
        // original requester instead of here
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
            respond_or_log_error(respond_to, Ok(message));
        }
    }

    async fn handle_update_or_query(
        &mut self,
        message: Message<f64>,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        if let Message::Update { path, .. } | Message::Query { path } = &message {
            // resurrect and forward if this is either Update or Query

            let mut actor_is_in_init = false;

            let actor = self.actors.entry(path.clone()).or_insert_with(|| {
                actor_is_in_init = true;
                // TODO: look up the gene by path
                // TODO: look up the gene by path
                // TODO: look up the gene by path
                // TODO: look up the gene by path
                // TODO: look up the gene by path
                let gene = Box::new(GuageAndAccumGene {
                    ..Default::default()
                });
                state_actor::new(path.clone(), 8, gene, None)
            });

            if let Some(store_actor) = &self.store_actor {
                if actor_is_in_init {
                    match actor.integrate(String::from(path), store_actor).await {
                        Ok(_) => {
                            // actor has read its journal
                        }
                        Err(e) => {
                            log::error!("can not load actor {e} from journal");
                        }
                    }
                }
            }

            let journaled: bool = match message.clone() {
                Message::Update { path: _, .. } => {
                    journal_message(message.clone(), &self.store_actor).await
                }
                Message::Query { path: _, .. } => true,
                m => {
                    log::warn!("unexpected message: {m}");
                    false
                }
            };

            handle_post_jrnl_procesing(journaled, message, respond_to, actor, &self.output).await;
        }
    }

    fn new(
        namespace: String,
        receiver: mpsc::Receiver<Envelope<f64>>,
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
