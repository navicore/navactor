use std::collections::HashMap;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct MessageEnvelope {
    pub message: Message,
    pub respond_to_opt: Option<oneshot::Sender<Message>>,
}

#[derive(Debug)]
pub enum Message {
    DefineCmd { spec: String },
    ReadAllCmd {},
    PrintOneCmd { text: String },
    UpdateCmd { values: HashMap<i32, f64> },
    IsCompleteMsg {},
}
