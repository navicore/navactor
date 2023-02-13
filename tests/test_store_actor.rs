use glob::glob;
use navactor::actor::Handle;
use navactor::director;
use navactor::json_decoder;
use navactor::message::Envelope;
use navactor::message::Message;
use navactor::message::MtHint;
use navactor::stdout_actor;
use navactor::store_actor_sqlite;
use std::fs;
use test_log::test;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

async fn setup_actors(db_file_prefix: String, namespace: String) -> Handle {
    let output_actor = stdout_actor::new(8);

    let store_actor = store_actor_sqlite::new(8, db_file_prefix, false, true);

    let director_w_persist = director::new(&namespace, 8, Some(output_actor), Some(store_actor));

    json_decoder::new(8, director_w_persist)
}

async fn shutdown_actors(json_decoder_actor: Handle) {
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
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::expect_used))]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_write_jrnl() {
    let namespace = String::from("/actors");
    let db_file_prefix = format!("/tmp/{namespace}");

    log::debug!("deleting db files before starting test...");
    for entry in glob(&format!("{db_file_prefix}.db*")).unwrap() {
        let path = entry.unwrap();
        log::debug!("deleting {path:?} before starting store test");
        fs::remove_file(path).unwrap();
    }

    let get_actor_one_file = "tests/data/get_actors_one_state.json";
    let get_actor_one_json = match fs::read_to_string(get_actor_one_file) {
        Ok(text) => text,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };

    let ob_1_3_file = "tests/data/single_observation_1_3.json";
    let ob_1_3_json = match fs::read_to_string(ob_1_3_file) {
        Ok(text) => text,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };

    let ob_2_3_file = "tests/data/single_observation_2_3.json";
    let _ob_2_3_json = match fs::read_to_string(ob_2_3_file) {
        Ok(text) => text,
        Err(e) => {
            println!("Error reading file: {}", e);
            return;
        }
    };

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // create actors and new db file
        let json_decoder_actor = setup_actors(db_file_prefix.clone(), namespace.clone()).await;

        // insert 1 update to the new db
        let cmd = Message::TextMsg {
            text: ob_1_3_json,
            hint: MtHint::Update,
        };
        let r = json_decoder_actor.tell(cmd).await;
        assert_eq!(r.ok(), Some(()));

        // stop actors and close db
        shutdown_actors(json_decoder_actor).await;

        // create actors and use previously created db file
        let json_decoder_actor = setup_actors(db_file_prefix, namespace).await;

        // query state of actor one from above updates
        let cmd = Message::TextMsg {
            text: get_actor_one_json,
            hint: MtHint::Query,
        };

        match json_decoder_actor.ask(cmd).await {
            Ok(r) => {
                if let Message::StateReport {
                    datetime: _,
                    path,
                    values,
                } = r
                {
                    assert_eq!(path, "/actors/one");
                    let keys: Vec<&i32> = values.keys().collect();
                    assert_eq!(keys.len(), 1);
                    assert_eq!(values.get(&3).unwrap(), &3.0);
                } else {
                    assert!(false, "bad response from output actor: {r:?}");
                }
            }
            Err(e) => {
                assert!(false, "{e}");
            }
        };
        // stop actors and close db
        shutdown_actors(json_decoder_actor).await;
    });
}
#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_read_jrnl() {
    //TODO
}
