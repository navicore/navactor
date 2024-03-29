use navactor::actors::director;
use navactor::actors::message::Envelope;
use navactor::actors::message::Message;
use navactor::actors::message::MtHint;
use navactor::io::json_decoder;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

/**
 * Create a director actor factory and send it json via `Message::Content`.
 */
#[cfg_attr(feature = "cargo-clippy", allow(clippy::expect_used))]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_actor_tell() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let director = director::new(&String::from("/"), 8, None, None);

        let json_decoder_actor = json_decoder::new(8, director); // parse input

        let cmd = Message::Content {
            path: None,
            hint: MtHint::Update,
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1, \"2\": 2, \"3\": 3} }"),
        };
        let r = json_decoder_actor.tell(cmd).await;
        assert_eq!(r.ok(), Some(()));

        let (send, recv) = oneshot::channel();

        let message = Message::EndOfStream {};

        let envelope = Envelope {
            message,
            respond_to: Some(send),
            ..Default::default()
        };

        let r = json_decoder_actor.send(envelope).await;
        assert_eq!(r.ok(), Some(()));

        let result = recv.await;

        let result_message = result.expect("failed with RecvErr");

        assert!(matches!(result_message, Ok(Message::EndOfStream {}),));
    });
}
