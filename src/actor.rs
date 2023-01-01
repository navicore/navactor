use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;

// TODO see if generics can enforce enum specific responses in ask and limit supported msgs in tell
// TODO see if generics can enforce enum specific responses in ask and limit supported msgs in tell
// TODO see if generics can enforce enum specific responses in ask and limit supported msgs in tell
// TODO see if generics can enforce enum specific responses in ask and limit supported msgs in tell

#[async_trait]
pub trait Actor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope);
}

#[async_trait]
pub trait ActorHandle {
    // system function - used by actor implementations directly only to forward "respond_to"
    // handles for workflow orchestration - TODO: need a cleaner way....
    async fn send(&self, envelope: MessageEnvelope);
    async fn tell(&self, msg: Message);
    async fn ask(&self, msg: Message) -> Message;
}
