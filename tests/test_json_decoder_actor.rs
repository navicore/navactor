use navactor::utils::nvtime::extract_datetime;
use navactor::io::json_decoder;
use navactor::actors::message::Envelope;
use navactor::actors::message::Message;
use navactor::actors::message::MtHint;
use navactor::io::stdout_actor;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tracing::debug;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_json_decode() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        let output_actor = stdout_actor::new(8);
        let json_decoder_actor = json_decoder::new(8, output_actor); // parse input

        let cmd = Message::Content {
            hint: MtHint::Update,
            path: None,
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.9, \"2\": 2.9} }"),
        };
        let r = json_decoder_actor.tell(cmd.clone()).await;
        assert_eq!(r.ok(), Some(()));
        
        match json_decoder_actor.ask(cmd).await {
            Ok(r) => {
                if let Message::Observations { datetime, path, values } = r {
                    assert_eq!(path, "/actors");
                    let keys: Vec<&i32> = values.keys().collect();
                    assert_eq!(keys.len(), 2);
                    assert_eq!(values.get(&1).unwrap(), &1.9);
                    assert_eq!(values.get(&2).unwrap(), &2.9);
                    let dt = extract_datetime("2023-01-11T23:17:57+0000").unwrap();
                    assert_eq!(dt, datetime);
                    let baddt = extract_datetime("2022-01-11T23:17:57+0000").unwrap();
                    assert_ne!(baddt, datetime);
                } else {
                    assert!(false, "bad response from output actor: {r:?}");
                }
            }
            Err(e) => {
                assert!(false, "{e}");
            }
        }
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

        debug!("result_message: {:?}", result_message);

        assert!(matches!(result_message, Ok(Message::EndOfStream {}),));

    });
}
