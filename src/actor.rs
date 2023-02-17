use crate::message::create_init_lifecycle;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::NvError;
use crate::message::NvResult;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

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
