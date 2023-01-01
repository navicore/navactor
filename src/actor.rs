use crate::message::Message;
use async_trait::async_trait;

#[async_trait]
pub trait Actor {
    async fn handle_message(&mut self, msg: Message);
}

#[async_trait]
pub trait ActorHandle {
    async fn tell(&self, msg: Message);
    async fn ask(&self, msg: Message) -> Message;
}
