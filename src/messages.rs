use tokio::sync::oneshot;

pub trait Respondable {
    fn respond_to(&self) -> &Option<oneshot::Sender<ActorMessage>>;
}

#[derive(Debug)]
pub enum ActorMessage {
    DefineCmd {
        spec: String,
        respond_to_opt: Option<oneshot::Sender<ActorMessage>>,
    },
    ReadAllCmd {
        respond_to_opt: Option<oneshot::Sender<ActorMessage>>,
    },
    PrintOneCmd {
        text: String,
    },
    // ErrorMsg {
    //     reason: String,
    // },
    IsCompleteMsg {
        respond_to_opt: Option<oneshot::Sender<ActorMessage>>,
    },
}

impl Respondable for ActorMessage {
    fn respond_to(&self) -> &Option<oneshot::Sender<ActorMessage>> {
        match self {
            ActorMessage::DefineCmd { respond_to_opt, .. } => &respond_to_opt,
            ActorMessage::ReadAllCmd { respond_to_opt, .. } => &respond_to_opt,
            ActorMessage::IsCompleteMsg { respond_to_opt, .. } => &respond_to_opt,
            _ => &None,
        }
    }
}
