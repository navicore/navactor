use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ActorMessage {
    ReadAllCmd { respond_to: oneshot::Sender<u32> },
    PrintOneCmd { text: String },
    IsCompleteMsg { respond_to: oneshot::Sender<u32> },
}
