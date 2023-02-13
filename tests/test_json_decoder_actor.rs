use navactor::nvtime::extract_datetime;
use navactor::json_decoder;
use navactor::message::Envelope;
use navactor::message::Message;
use navactor::message::MtHint;
use navactor::stdout_actor;
use test_log::test;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_json_decode() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        let output_actor = stdout_actor::new(8);
        let json_decoder_actor = json_decoder::new(8, output_actor); // parse input

        let cmd = Message::TextMsg {
            hint: MtHint::Update,
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.9, \"2\": 2.9} }"),
        };
        let r = json_decoder_actor.tell(cmd.clone()).await;
        assert_eq!(r.ok(), Some(()));
        
        match json_decoder_actor.ask(cmd).await {
            Ok(r) => {
                if let Message::Update { datetime, path, values } = r {
                    assert_eq!(path, "/actors");
                    let keys: Vec<&i32> = values.keys().collect();
                    assert_eq!(keys.len(), 2);
                    assert_eq!(values.get(&1).unwrap(), &1.9);
                    assert_eq!(values.get(&2).unwrap(), &2.9);
                    let dt = extract_datetime("2023-01-11T23:17:57+0000");
                    assert_eq!(dt, datetime);
                    let baddt = extract_datetime("2022-01-11T23:17:57+0000");
                    assert_ne!(baddt, datetime);
                } else {
                    assert!(false, "bad response from output actor: {r:?}");
                }
            }
            Err(e) => {
                assert!(false, "{e}");
            }
        }
        //assert_eq!(r.ok(), Some(()));

        //
        // shutdown
        //
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

        log::debug!("result_message: {:?}", result_message);

        assert!(matches!(result_message, Ok(Message::EndOfStream {}),));

    });
}
