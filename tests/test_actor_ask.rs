use navactor::director;
use navactor::json_decoder;
use navactor::message::Message;
use navactor::state_actor;
use std::collections::HashMap;
use test_log::test;
use time::OffsetDateTime;
use tokio::runtime::Runtime;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_actor_ask() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let state_actor = state_actor::new("/".to_string(), 8, None); // parse input

        // set an initial state
        let mut values = HashMap::new();
        values.insert(1, 1.9);
        values.insert(2, 2.9);

        let cmd = Message::Update {
            path: String::from("/"),
            datetime: OffsetDateTime::now_utc(),
            values,
        };
        let r = state_actor.tell(cmd).await;
        assert_eq!(r.ok(), Some(()));

        // update state
        let mut values = HashMap::new();
        values.insert(1, 1.8);
        let datetime = OffsetDateTime::now_utc();
        let cmd = Message::Update {
            path: String::from("/"),
            datetime,
            values,
        };
        let reply = state_actor.ask(cmd).await;

        assert!(matches!(
            reply,
            Ok(Message::StateReport {
                datetime: _,
                path: _,
                values: _
            }),
        ));

        if let Ok(Message::StateReport {
            datetime: _,
            path: _,
            values: new_values,
        }) = reply
        {
            // ensure that the initial state for 2 is still there but that the initial state for 1
            // was updated
            let v1 = new_values.get(&1);
            assert_eq!(v1.unwrap(), &1.8);
            let v2: Option<&f64> = new_values.get(&2);
            assert_eq!(v2.unwrap(), &2.9);
        }
    });
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_decoder_ask() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        let director = director::new(String::from("/"), 8, None, None);
        let json_decoder_actor = json_decoder::new(8, director); // parse input

        // init state
        let cmd = Message::PrintOneCmd {
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.9, \"2\": 2.9} }"),
        };
        let r = json_decoder_actor.tell(cmd).await;
        assert_eq!(r.ok(), Some(()));

        // update state
        let cmd = Message::PrintOneCmd {
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.8} }"),
        };
        let reply = json_decoder_actor.ask(cmd).await;

        assert!(matches!(
            reply,
            Ok(Message::StateReport {
                datetime: _,
                path: _,
                values: _
            }),
        ));

        if let Ok(Message::StateReport {
            datetime: _,
            path: _,
            values: new_values,
        }) = reply
        {
            // ensure that the initial state for 2 is still there but that the initial state for 1
            // was updated
            let v1 = new_values.get(&1);
            assert_eq!(v1.unwrap(), &1.8);
            let v2 = new_values.get(&2);
            assert_eq!(v2.unwrap(), &2.9);
        }
    });
}
