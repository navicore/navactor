use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use crate::nvtime::OffsetDateTimeWrapper;
use async_trait::async_trait;
use serde_json::{from_str, to_string};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use time::OffsetDateTime;
use tokio::sync::mpsc;

pub struct StoreActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub dbconn: Option<sqlx::SqlitePool>,
    pub namespace: String,
}

#[async_trait]
impl Actor for StoreActor {
    async fn stop(&mut self) {
        if let Some(c) = &self.dbconn {
            c.close().await;
        }
    }
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            stream_to,
            ..
        } = envelope;

        match message {
            Message::Update {
                path,
                datetime,
                values,
            } => {
                self.insert_update(&path, datetime, values).await;

                if let Some(respond_to) = respond_to {
                    respond_to
                        .send(Message::EndOfStream {})
                        .expect("cannot respond to 'ask' with confirmation");
                }
            }

            Message::LoadCmd { path } => {
                // play jrnl to resurected actor so that they can process 'next_message'
                if let Some(stream_to) = stream_to {
                    log::trace!("handling LoadCmd for {}", path);
                    if let Some(dbconn) = &self.dbconn {
                        let rows: Vec<Message> = self.get_jrnl(dbconn, &path).await;
                        log::trace!(
                            "handling LoadCmd jrnl for {} items count: {}",
                            path,
                            rows.len()
                        );
                        for message in rows {
                            stream_to
                                .send(message)
                                .await
                                .expect("can not send jrnl event from helper");
                        }
                    }
                    stream_to
                        .send(Message::EndOfStream {})
                        .await
                        .expect("can not integrate from helper");

                    stream_to.closed().await;
                }
            }
            m => log::warn!("unexpected: {:?}", m),
        }
    }
}

impl StoreActor {
    /// actor private constructor
    fn new(
        receiver: mpsc::Receiver<MessageEnvelope>,
        dbconn: Option<sqlx::SqlitePool>,
        namespace: String,
    ) -> Self {
        StoreActor {
            receiver,
            dbconn,
            namespace,
        }
    }

    /// retrieve the time series of events (observations) for the actor that is being resurrected
    async fn get_jrnl(&self, dbconn: &SqlitePool, path: &String) -> Vec<Message> {
        sqlx::query("SELECT timestamp, values_str FROM updates WHERE path = ?")
            .bind(path.clone())
            .try_map(|row: sqlx::sqlite::SqliteRow| {
                let data_parsed: OffsetDateTimeWrapper = from_str(row.get(0)).unwrap();
                let values: HashMap<i32, f64> =
                    serde_json::from_str(row.try_get(1).expect("cannot extract values"))
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))
                        .expect("cannot return values");

                Ok(Message::Update {
                    path: path.clone(),
                    datetime: data_parsed.datetime,
                    values,
                })
            })
            .fetch_all(dbconn)
            .await
            .expect("cannot load from db")
    }

    /// record the latest event in the actors state
    async fn insert_update(
        &self,
        path: &String,
        datetime: OffsetDateTime,
        values: HashMap<i32, f64>,
    ) {
        // store this is a db with the key as 'path'
        if let Some(dbconn) = &self.dbconn {
            let dt_wrapper = OffsetDateTimeWrapper { datetime };
            let datetime_str = to_string(&dt_wrapper).unwrap();

            sqlx::query("INSERT INTO updates (path, timestamp, values_str) VALUES (?,?,?)")
                .bind(path.clone())
                .bind(datetime_str)
                .bind(serde_json::to_string(&values).expect("cannot serialize values"))
                .execute(dbconn)
                .await
                .expect("db insert failed");
            log::trace!("jrnled Update for {}", path);
        } else {
            log::error!("db conn not set");
        }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize, namespace: String) -> ActorHandle {
    async fn init_db(namespace: String) -> sqlx::SqlitePool {
        // TODO: default is memory but file comes from nv cli
        let db_url_string: String = format!("{}.db", namespace);
        let db_url: &str = &db_url_string;
        let db_path = Path::new(db_url);
        if !db_path.exists() {
            match File::create(db_url) {
                Ok(_) => log::debug!("File {} has been created", db_url),
                Err(e) => log::warn!("Failed to create file {}: {}", db_url, e),
            }
        }
        let dbconn = SqlitePool::connect(db_url).await.expect("");

        // Create table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS updates (
              path TEXT NOT NULL,
              timestamp TEXT NOT NULL,
              values_str TEXT NOT NULL
        )",
        )
        .execute(&dbconn)
        .await
        .expect("cannot create table");

        dbconn
    }

    async fn start(mut actor: StoreActor, namespace: String) {
        let dbconn = init_db(namespace).await;
        actor.dbconn = Some(dbconn);
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
        actor.stop().await;
    }

    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StoreActor::new(receiver, None, namespace.clone());
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor, namespace));
    actor_handle
}
