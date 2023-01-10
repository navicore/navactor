use nv::json_value_decoder_actor;
use nv::message::Message;
use nv::message::MessageEnvelope;
use nv::state_actor;
use test_log::test;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

#[test]
fn test_actor_tell() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let state_actor = state_actor::new(8, None); // parse input
        let json_decoder_actor = json_value_decoder_actor::new(8, state_actor); // parse input

        let cmd = Message::PrintOneCmd {
            text: "{\"1\": 1.0}".to_string(),
        };
        json_decoder_actor.tell(cmd).await;

        let (send, recv) = oneshot::channel();

        let message = Message::IsCompleteMsg {};
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: Some(send),
            ..Default::default()
        };
        json_decoder_actor.send(envelope).await;
        let result = recv.await;
        let result_message = result.expect("failed with RecvErr");
        assert!(matches!(result_message, Message::IsCompleteMsg {},));
    });
}
