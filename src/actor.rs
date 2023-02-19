use crate::message::create_init_lifecycle;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::NvError;
use crate::message::NvResult;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

///This here be a Rust code for actors, mateys! It defines the interface for actors in Rust, as
///well as an API called `ActorHandle` that allows you to communicate with actors by sending and
///receiving messages.
///
///This Rust code includes a few helper methods, such as `respond_or_log_error`, which is used by
///most actors to respond to messages if they are of the `ask` type, and to log an error if the
///response cannot be sent.
///
///The `Handle` struct is used to create the API for actors, and it includes methods such as
///`send`, `tell`, `ask`, and `integrate`. These methods allow you to send messages to actors and
///receive responses, as well as to coordinate the instantiation of a new actor with the help of
///another actor.
///
///The `Actor` trait defines the functions that each actor must implement, namely `handle_envelope`
///and stop. The former is used to handle incoming messages, while the latter is used to stop the
///actor.
///
///The Rust code also includes some type aliases, such as `State<T>`, which is a hash map that maps
///an `i32` to a type `T`.
///
///This Rust code uses Rust's `async_trait` library, which allows you to write asynchronous code
///using traits.

/// in-mem state for an actor
pub type State<T> = std::collections::HashMap<i32, T>;

/// all actors must implement this trait
#[async_trait]
pub trait Actor {
    /// the function to implement per actor
    async fn handle_envelope(&mut self, envelope: Envelope<f64>);
    async fn stop(&self);
}

/// `ActorHandle` is the API for all actors
pub struct Handle {
    #[doc(hidden)]
    pub sender: mpsc::Sender<Envelope<f64>>,
}

/// `ActorHandle` is the API for all actors via `ask` and `tell`
impl<'a> Handle {
    // INTERNAL: currently used by builtins (nv actors) implementing
    // actors that forward respond_to in workflows.
    #[doc(hidden)]
    pub async fn send(&self, envelope: Envelope<f64>) -> NvResult<()> {
        self.sender.send(envelope).await.map_err(|e| NvError {
            reason: e.to_string(),
        })
    }

    /// fire and forget
    ///
    /// # Errors
    /// Returns [`NvError`](../message/struct.NvError.html) if the
    /// message is not received by the target actor
    pub async fn tell(&self, message: Message<f64>) -> NvResult<()> {
        let envelope = Envelope {
            message,
            respond_to: None,
            ..Default::default()
        };

        log::trace!("tell sending envelope {envelope:?}");
        self.send(envelope).await
    }

    /// request <-> response
    ///
    /// # Errors
    ///
    /// Returns [`NvError`](../message/struct.NvError.html) if the
    /// message is not received and replied to by the target actor
    pub async fn ask(&self, message: Message<f64>) -> NvResult<Message<f64>> {
        let (send, recv) = oneshot::channel();

        let envelope = Envelope {
            message,
            respond_to: Some(send),
            ..Default::default()
        };

        log::trace!("ask sending envelope: {envelope:?}");
        match self.send(envelope).await {
            Ok(_) => recv.await.map_err(|e| NvError {
                reason: e.to_string(),
            })?,

            Err(e) => Err(e),
        }
    }

    /// call to coordinate the instantiation of a new acotr with the help
    /// of another actor - usually a datastore journal service
    ///
    /// # Errors
    ///
    /// Returns [`NvError`](../message/struct.NvError.html) if the
    /// two actors don't exchange lifecycle info
    pub async fn integrate(&self, path: String, helper: &Self) -> NvResult<Message<f64>> {
        type ResultSender = Sender<NvResult<Message<f64>>>;
        type ResultReceiver = oneshot::Receiver<NvResult<Message<f64>>>;
        type SendReceivePair = (ResultSender, ResultReceiver);

        let (send, recv): SendReceivePair = oneshot::channel();

        let (init_cmd, load_cmd) = create_init_lifecycle(path, 8, send);

        helper.send(load_cmd).await.map_err(|e| NvError {
            reason: e.to_string(),
        })?;

        self.send(init_cmd).await.map_err(|e| NvError {
            reason: e.to_string(),
        })?;

        recv.await.map_err(|e| NvError {
            reason: e.to_string(),
        })?
    }

    // ActorHandle constructor is an internal API use in the convenience functions
    // of the various per-actor ActorHandle impls
    #[doc(hidden)]
    #[must_use]
    pub const fn new(sender: mpsc::Sender<Envelope<f64>>) -> Self {
        Self { sender }
    }
}

/// utility function most actors need to reply if a message is an 'ask'
pub fn respond_or_log_error(
    respond_to: Option<Sender<NvResult<Message<f64>>>>,
    result: NvResult<Message<f64>>,
) {
    {
        if let Some(respond_to) = respond_to {
            match respond_to.send(result) {
                Ok(_) => (),
                Err(err) => {
                    log::error!("Cannot respond to 'ask' with confirmation: {:?}", err);
                }
            }
        }
    }
}
