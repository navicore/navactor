use tokio::sync::oneshot;

pub trait Respondable {
    fn respond_to(&self) -> &Option<oneshot::Sender<Message>>;
}

struct MessageEnvelope {
    message: Message,
    respond_to_opt: Option<oneshot::Sender<Message>>,
}

#[derive(Debug)]
pub enum Message {
    DefineCmd {
        spec: String,
        respond_to_opt: Option<oneshot::Sender<Message>>,
    },
    ReadAllCmd {
        respond_to_opt: Option<oneshot::Sender<Message>>,
    },
    PrintOneCmd {
        text: String,
    },
    // ErrorMsg {
    //     reason: String,
    // },
    IsCompleteMsg {
        respond_to_opt: Option<oneshot::Sender<Message>>,
    },
}

impl Respondable for Message {
    fn respond_to(&self) -> &Option<oneshot::Sender<Message>> {
        match self {
            Message::DefineCmd { respond_to_opt, .. } => &respond_to_opt,
            Message::ReadAllCmd { respond_to_opt, .. } => &respond_to_opt,
            Message::IsCompleteMsg { respond_to_opt, .. } => &respond_to_opt,
            _ => &None,
        }
    }
}
