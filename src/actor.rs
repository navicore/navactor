use crate::messages::ActorMessage;
use async_trait::async_trait;

#[async_trait]
pub trait Actor {
    async fn handle_message(&mut self, msg: ActorMessage);
}

#[async_trait]
pub trait ActorHandle {
    async fn send(&self, msg: ActorMessage);
}
