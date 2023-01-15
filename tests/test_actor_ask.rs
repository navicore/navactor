use chrono::Utc;
use nv::graph_director;
use nv::json_update_decoder_actor;
use nv::message::Message;
use nv::state_actor;
use std::collections::HashMap;
use test_log::test;
use tokio::runtime::Runtime;

#[test]
fn test_actor_ask() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let state_actor = state_actor::new("/".to_string(), 8, None); // parse input

        // set an initial state
        let mut values = HashMap::new();
        values.insert(1, 1.9);
        values.insert(2, 2.9);

        let path = String::from("/");
        let cmd = Message::UpdateCmd {
            datetime: Utc::now(),
            path,
            values,
        };
        state_actor.tell(cmd).await;

        // update state
        let mut values = HashMap::new();
        values.insert(1, 1.8);
        let path = String::from("/");
        let datetime = Utc::now();
        let cmd = Message::UpdateCmd {
            datetime,
            path,
            values,
        };
        let reply = state_actor.ask(cmd).await;

        assert!(matches!(
            reply,
            Message::StateReport {
                datetime: _,
                path: _,
                values: _
            },
        ));

        if let Message::StateReport {
            datetime: _,
            path: _,
            values: new_values,
        } = reply
        {
            // ensure that the initial state for 2 is still there but that the initial state for 1
            // was updated
            let v1 = new_values.get(&1).expect("should be there");
            assert_eq!(v1, &1.8);
            let v2 = new_values.get(&2).expect("should be there");
            assert_eq!(v2, &2.9);
        }
    })
}

#[test]
fn test_decoder_ask() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        let graph_director = graph_director::new(String::from("/"), 8, None);
        let json_decoder_actor = json_update_decoder_actor::new(8, graph_director); // parse input

        // init state
        let cmd = Message::PrintOneCmd {
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.9, \"2\": 2.9} }"),
        };
        json_decoder_actor.tell(cmd).await;

        // update state
        let cmd = Message::PrintOneCmd {
            text: String::from("{ \"path\": \"/actors\", \"datetime\": \"2023-01-11T23:17:57+0000\", \"values\": {\"1\": 1.8} }"),
        };
        let reply = json_decoder_actor.ask(cmd).await;

        assert!(matches!(
            reply,
            Message::StateReport {
                datetime: _,
                path: _,
                values: _
            },
        ));

        if let Message::StateReport {
            datetime: _,
            path: _,
            values: new_values,
        } = reply
        {
            // ensure that the initial state for 2 is still there but that the initial state for 1
            // was updated
            let v1 = new_values.get(&1).expect("should be there");
            assert_eq!(v1, &1.8);
            let v2 = new_values.get(&2).expect("should be there");
            assert_eq!(v2, &2.9);
        }
    })
}
