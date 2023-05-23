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
//!creates a new handle for the `Director` actor, which is used to send messages to the
//!`Director`. The `Director` is also responsible for creating and storing graph edges to
//!support arbitrary paths.
//!
//!The `Director` uses other Rust crates and libraries, such as `tokio`, `async_trait`,
//!`std::collections::HashMap`, and others.

use crate::actors::actor::respond_or_log_error;
use crate::actors::actor::Actor;
use crate::actors::actor::Handle;
use crate::actors::message::create_init_lifecycle;
use crate::actors::message::Envelope;
use crate::actors::message::Message;
use crate::actors::message::MtHint;
use crate::actors::message::NvError;
use crate::actors::message::NvResult;
use crate::actors::state_actor;
use crate::genes::accum_gene::AccumGene;
use crate::genes::gauge_and_accum_gene::GaugeAndAccumGene;
use crate::genes::gauge_gene::GaugeGene;
use crate::genes::gene::Gene;
use crate::genes::gene::GeneType;
use async_trait::async_trait;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::trace;
use tracing::warn;

// TODO:
// use rust std Path to update and persist petgraph graph Edges and
// lookup/upsert actor for each input record msg send

/// This struct represents a graph director that creates a graph and instantiates all the actors
/// that it is forwarding commands to. The director also accepts metadata to create and store graph
/// edges to support arbitrary paths.
#[derive(Debug)]
pub struct Director {
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub store_actor: Option<Handle>,
    pub output: Option<Handle>,
    pub actors: HashMap<String, Handle>,
    pub gene_path_map: HashMap<String, GeneType>,
    namespace: String,
}

#[async_trait]
impl Actor for Director {
    // This function is called when an envelope is received by the Director actor
    #[allow(clippy::too_many_lines)]
    #[instrument]
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        trace!(
            "director namespace {} handling_envelope {envelope}",
            self.namespace
        );
        let Envelope {
            message,
            respond_to,
            stream_from,
            ..
        } = envelope;

        match &message {
            Message::InitCmd { .. } => {
                trace!("{} init started...", self.namespace);
                // this is an init so read your old mappings
                if let Some(mut stream_from) = stream_from {
                    let mut count = 0;

                    while let Some(message) = stream_from.recv().await {
                        match &message {
                            Message::EndOfStream {} => {
                                break;
                            }
                            Message::GeneMapping { path, gene_type } => {
                                count += 1;
                                self.gene_path_map.insert(path.clone(), *gene_type);
                            }
                            _ => {}
                        }
                    }
                    trace!("{} init closing stream.", self.namespace);
                    stream_from.close();

                    debug!(
                        "{} finished init with {} remembered mappings",
                        self.namespace, count
                    );

                    respond_or_log_error(respond_to, Ok(Message::EndOfStream {}));
                }
            }

            // maintain the path-to-gene mappings
            Message::GeneMapping { path, gene_type } => {
                debug!("setting new mapping: {path} {gene_type}");
                self.handle_gene_mapping(path, *gene_type, message.clone(), respond_to)
                    .await;
            }

            Message::Content {
                path,
                text: _,
                hint,
            } if hint == &MtHint::GeneMappingQuery => {
                debug!("getting mapping for {path:?}");
                self.handle_gene_mapping_query(
                    path.clone().unwrap_or_default().as_str(),
                    respond_to,
                );
            }

            // If the message is an update or a query, handle it by calling the corresponding function
            Message::Observations { path, .. } => {
                self.handle_update_or_query(&path.clone(), message, respond_to)
                    .await;
            }
            Message::Query { path, hint, .. } if hint == &MtHint::State => {
                // TODO: let query get all actor paths if path ends with a "/" otherwise get state
                self.handle_update_or_query(&path.clone(), message, respond_to)
                    .await;
            }
            Message::Query { path, hint, .. } if hint == &MtHint::GeneMapping => {
                let path_string = String::from(path);

                let prefix_string = if path_string.ends_with('/') {
                    path_string
                } else {
                    path_string + "/"
                };
                let prefix: &str = prefix_string.as_str();

                let mut clean_prefix_string: String = prefix_string.clone();
                if clean_prefix_string.ends_with('/') {
                    clean_prefix_string.pop();
                };
                let clean_prefix: &str = clean_prefix_string.as_str();

                let mut response: String = String::new();
                let pairs: Vec<(&String, &GeneType)> = self
                    .gene_path_map
                    .iter()
                    .filter(|(key, _)| {
                        key == &path || key == &clean_prefix || key.starts_with(prefix)
                    })
                    .collect();
                for (key, val) in pairs {
                    if !response.is_empty() {
                        response += "\n";
                    }
                    response += format!("{key} -> {val}").as_str();
                }
                if response.is_empty() {
                    response = String::from("<not set>");
                }
                let msg = Message::Content {
                    path: None,
                    text: response,
                    hint: MtHint::GeneMapping,
                };
                self.forward_report(msg, respond_to).await;
            }

            // If the message is an EndOfStream message, forward it to the output actor
            // or send the response directly to the original requester
            Message::EndOfStream {} => self.handle_end_of_stream(message, respond_to).await,
            // If the message is unexpected, log an error and respond with an NvError
            m => {
                let emsg = format!("unexpected message: {m}");
                error!("{emsg}");
                respond_or_log_error(respond_to, Err(NvError { reason: emsg }));
            }
        }
    }

    #[instrument]
    async fn stop(&self) {}
    #[instrument]
    async fn start(&mut self) {
        info!("starting");
        if let Some(store_actor) = &self.store_actor {
            type ResultSender = Sender<NvResult<Message<f64>>>;
            type ResultReceiver = oneshot::Receiver<NvResult<Message<f64>>>;
            type SendReceivePair = (ResultSender, ResultReceiver);

            let (send, recv): SendReceivePair = oneshot::channel();

            let (init_cmd, load_cmd) =
                create_init_lifecycle(self.namespace.clone(), 8, send, MtHint::GeneMapping);

            store_actor
                .send(load_cmd)
                .await
                .map_err(|e| {
                    error!("cannot start director: {e}");
                })
                .ok();

            self.handle_envelope(init_cmd).await;

            match recv.await {
                Ok(_) => {}
                Err(e) => error!("cannot start director because of store error: {e}"),
            }
        }
    }
}

// wrapper to support flow when no persistence is configured
#[instrument]
async fn journal_message(
    message: Message<f64>,
    store_actor: &Option<Handle>,
) -> Result<Message<f64>, NvError> {
    if let Some(store_actor) = store_actor {
        trace!("journal_message {message}");
        // jrnl the new msg
        store_actor.ask(message.clone()).await
    } else {
        // If journaling is explicitly not supported in this deployment return success
        trace!("journaling messages is disabled - proceeding ok");
        Ok(Message::Persisted)
    }
}

#[instrument]
async fn forward_actor_result(result: NvResult<Message<f64>>, output: &Option<Handle>) {
    //forward to optional output
    trace!("forward_actor_result");
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
                    error!("can not forward: {e:?}");
                }
            }
        }
    }
}

#[instrument]
async fn write_jrnl(
    message: Message<f64>,
    store_actor: &Option<Handle>,
) -> Result<Message<f64>, NvError> {
    match message.clone() {
        Message::Observations { path: _, .. } => {
            trace!("write_jrnl");
            journal_message(message.clone(), store_actor).await
        }
        Message::Query { path: _, .. } => Ok(Message::Persisted),
        m => {
            warn!("unexpected message: {m}");
            Err(NvError {
                reason: "unexpected msg".to_string(),
            })
        }
    }
}

#[instrument]
async fn send_to_actor(
    message: Message<f64>,
    respond_to: Option<Sender<NvResult<Message<f64>>>>,
    actor: &Handle,
    output: &Option<Handle>,
) {
    trace!("send_to_actor sending to actor");
    //send message to the actor and support ask results
    let r = actor.ask(message).await;
    respond_or_log_error(respond_to, r.clone());

    //forward to optional output
    forward_actor_result(r, output).await;
}

fn get_gene(gene_type: GeneType) -> Box<dyn Gene<f64> + Send + Sync> {
    match gene_type {
        GeneType::Accum => Box::new(AccumGene {
            ..Default::default()
        }),
        GeneType::Gauge => Box::new(GaugeGene {
            ..Default::default()
        }),
        _ => Box::new(GaugeAndAccumGene {
            ..Default::default()
        }),
    }
}

/// actor private constructor
impl Director {
    fn handle_gene_mapping_query(
        &mut self,
        path: &str,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        if let Some(gt) = self.gene_path_map.get(path) {
            let msg = Message::GeneMapping {
                path: path.to_string(),
                gene_type: *gt,
            };
            respond_or_log_error(respond_to, Ok(msg));
        } else {
            let msg = Message::NotFound {
                path: path.to_string(),
            };
            respond_or_log_error(respond_to, Ok(msg));
        }
    }

    #[instrument]
    async fn handle_gene_mapping(
        &mut self,
        path: &str,
        gene_type: GeneType,
        message: Message<f64>, // for jrnl
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        debug!("new gene_mapping");
        self.gene_path_map.insert(String::from(path), gene_type);
        if let Some(store_actor) = &self.store_actor {
            let jrnl_msg = store_actor.ask(message.clone()).await;
            match jrnl_msg {
                Ok(_) => {
                    // live mapping updated and persisted
                    respond_or_log_error(respond_to, Ok(message));
                }
                Err(e) => respond_or_log_error(
                    respond_to,
                    Err(NvError {
                        reason: format!("{e}"),
                    }),
                ),
            }
        } else {
            // no persistence - all is fine
            respond_or_log_error(respond_to, Ok(message));
        }
    }

    #[instrument]
    async fn forward_report(
        &self,
        message: Message<f64>,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        if let Some(a) = &self.output {
            let senv = Envelope {
                message,
                respond_to,
                ..Default::default()
            };
            a.send(senv)
                .await
                .map_err(|e| {
                    error!("cannot send: {e:?}");
                })
                .ok();
        } else {
            respond_or_log_error(respond_to, Ok(message));
        }
    }

    #[instrument]
    async fn handle_end_of_stream(
        &self,
        message: Message<f64>,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        debug!("complete");

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
                    error!("cannot send: {e:?}");
                })
                .ok();
        } else {
            respond_or_log_error(respond_to, Ok(message));
        }
    }

    #[instrument]
    async fn handle_update_or_query(
        &mut self,
        path: &String,
        message: Message<f64>,
        respond_to: Option<Sender<NvResult<Message<f64>>>>,
    ) {
        // resurrect and forward if this is either Update or Query
        match self.actors.entry(path.clone()) {
            Entry::Vacant(entry) => {
                trace!("handle_update_or_query creating new or resurrected instance");

                //
                // BEGIN inline because of single mutable share compiler error when I put this
                // in Director impl and try to mut borrow self twice
                //

                let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                let mut current_path = String::new();
                let mut reg_gene_type = None;

                for component in &components {
                    current_path.push('/');
                    current_path.push_str(component);

                    if let Some(gt) = self.gene_path_map.get(&current_path) {
                        reg_gene_type = Some(*gt);
                    }
                }
                let gene_type = reg_gene_type.unwrap_or(GeneType::Gauge);

                //
                // END inline
                //

                let actor = state_actor::new(path.clone(), 8, get_gene(gene_type), None);
                if let Some(store_actor) = &self.store_actor {
                    actor
                        .integrate(String::from(path), store_actor, MtHint::Update)
                        .await
                        .map_err(|e| {
                            error!("can not load actor {e} from journal");
                        })
                        .ok();
                }
                let jrnled = write_jrnl(message.clone(), &self.store_actor).await;
                match jrnled {
                    Ok(Message::Persisted) => {
                        send_to_actor(message, respond_to, &actor, &self.output).await;
                    }
                    Ok(Message::ConstraintViolation) => {
                        respond_or_log_error(respond_to, Ok(Message::ConstraintViolation {}));
                    }
                    _ => {
                        respond_or_log_error(respond_to, jrnled);
                    }
                }
                entry.insert(actor); // put it where you can find it again
            }
            Entry::Occupied(entry) => {
                trace!("handle_update_or_query found live instance");
                let actor = entry.get();
                let jrnled = write_jrnl(message.clone(), &self.store_actor).await;
                // todo: return meaningful errors
                match jrnled {
                    Ok(Message::Persisted) => {
                        send_to_actor(message, respond_to, actor, &self.output).await;
                    }
                    Ok(Message::ConstraintViolation) => {
                        respond_or_log_error(respond_to, Ok(Message::ConstraintViolation {}));
                    }
                    _ => {
                        respond_or_log_error(respond_to, jrnled);
                    }
                };
            }
        };
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
            gene_path_map: HashMap::new(),
        }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(
    namespace: &str,
    bufsz: usize,
    output: Option<Handle>,
    store_actor: Option<Handle>,
) -> Handle {
    #[instrument]
    async fn start(mut actor: Director) {
        actor.start().await;
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = Director::new(namespace.to_string(), receiver, output, store_actor);

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor));

    debug!("{} started", namespace);
    actor_handle
}
