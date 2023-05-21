use glob::glob;
use navactor::actor::Handle;
use navactor::director;
use navactor::json_decoder;
use navactor::message::Message;
use navactor::message::MtHint;
use navactor::stdout_actor;
use navactor::store_actor_sqlite;
use std::fs;
use tokio::runtime::Runtime;
use tracing::debug;

async fn setup_actors(db_file_prefix: String, namespace: String) -> Handle {
    let output_actor = stdout_actor::new(8);

    // do not configured to tolerate collisions because the "allow dupes" setting uses envelope
    // time and that causes collisions due to sub-millisecond execution of the test.
    let store_actor = store_actor_sqlite::new(8, db_file_prefix, false, false);

    let director_w_persist = director::new(&namespace, 8, Some(output_actor), Some(store_actor));

    json_decoder::new(8, director_w_persist)
}

async fn shutdown_actors(json_decoder_actor: Handle) {
    let message = Message::EndOfStream {};

    let result_message = json_decoder_actor.ask(message).await;

    debug!("shutdown result_message: {:?}", result_message);
    assert!(matches!(result_message, Ok(Message::EndOfStream {}),));
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_store_gene_mapping() {
    let namespace = String::from("/gene_actors");
    let db_file_prefix = format!("/tmp/{namespace}");

    debug!("deleting db files before starting test...");
    for entry in glob(&format!("{db_file_prefix}.db*")).unwrap() {
        let path = entry.unwrap();
        debug!("deleting {path:?} before starting store test");
        fs::remove_file(path).unwrap();
    }

    let get_gene_mapping_file = "tests/data/gene_mapping_actors_accum.json";
    let get_gene_mapping_json = match fs::read_to_string(get_gene_mapping_file) {
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
        let cmd = Message::Content {
            text: get_gene_mapping_json,
            path: None,
            hint: MtHint::GeneMapping,
        };
        let r = json_decoder_actor.tell(cmd).await;
        assert_eq!(r.ok(), Some(()));

        // stop actors and close db
        shutdown_actors(json_decoder_actor).await;
    });
}
