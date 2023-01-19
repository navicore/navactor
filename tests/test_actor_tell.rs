use nv::director;
use nv::json_decoder;
use nv::message::Message;
use nv::message::MessageEnvelope;
use test_log::test;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

#[test]
fn test_actor_tell() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let director = director::new(String::from("/"), 8, None);
        let json_decoder_actor = json_decoder::new(8, director); // parse input

        let cmd = Message::PrintOneCmd {
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1, \"2\": 2, \"3\": 3} }"),
        };
        json_decoder_actor.tell(cmd).await;

        let (send, recv) = oneshot::channel();

        let message = Message::EndOfStream {};
        let envelope = MessageEnvelope {
            message,
            respond_to_opt: Some(send),
            ..Default::default()
        };
        json_decoder_actor.send(envelope).await;
        let result = recv.await;
        let result_message = result.expect("failed with RecvErr");
        log::debug!("result_message: {:?}", result_message);
        assert!(matches!(result_message, Message::EndOfStream {},));
    });
}
