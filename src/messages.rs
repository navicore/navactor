use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ActorMessage {
    ReadAllCmd {
        respond_to: oneshot::Sender<ActorMessage>,
    },
    PrintOneCmd {
        text: String,
    },
    IsCompleteMsg {
        respond_to_opt: Option<oneshot::Sender<ActorMessage>>,
    },
}
