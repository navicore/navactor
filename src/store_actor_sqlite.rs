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
            datetime: _,
            stream_to,
            stream_from: _,
            next_message: _,
            next_message_respond_to: _,
        } = envelope;
        match message {
            Message::Update {
                path,
                datetime,
                values,
            } => {
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
                if let Some(respond_to) = respond_to {
                    respond_to
                        .send(Message::EndOfStream {})
                        .expect("cannot respond to 'ask' with confirmation");
                }
            }
            Message::LoadCmd { path } => {
                if let Some(stream_to) = stream_to {
                    log::trace!("handling LoadCmd for {}", path);
                    // 1. query db with the path as key
                    // 2. write events to "stream_to"
                    // 3. write next_message to "stream_to"
                    // 4. write EndOfStream to "stream_to"
                    // 5. close stream_to
                    if let Some(dbconn) = &self.dbconn {
                        log::trace!("handling LoadCmd jrnl retrieval for {}", path);
                        let rows: Vec<Message> =
                            sqlx::query("SELECT timestamp, values_str FROM updates WHERE path = ?")
                                .bind(path.clone())
                                .try_map(|row: sqlx::sqlite::SqliteRow| {
                                    let datetime_str: String = row.get(0);
                                    let data_parsed: OffsetDateTimeWrapper =
                                        from_str(&datetime_str).unwrap();
                                    let datetime = data_parsed.datetime;
                                    let values: HashMap<i32, f64> = serde_json::from_str(
                                        row.try_get(1).expect("cannot extract values"),
                                    )
                                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))
                                    .expect("cannot return values");

                                    Ok(Message::Update {
                                        path: path.clone(),
                                        datetime,
                                        values,
                                    })
                                })
                                .fetch_all(dbconn)
                                .await
                                .expect("cannot load from db");
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

/// actor private constructor
impl StoreActor {
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
}

/// actor handle public constructor
pub fn new(bufsz: usize, namespace: String) -> ActorHandle {
    async fn init_db(namespace: String) -> sqlx::SqlitePool {
        // TODO: default is memory but file comes from nv cli
        let db_url_string: String = format!("{}.db", namespace);
        let db_url: &str = &db_url_string;
        let db_path = Path::new(db_url.clone());
        if !db_path.exists() {
            match File::create(db_url) {
                Ok(_) => log::debug!("File {} has been created", db_url),
                Err(e) => log::warn!("Failed to create file {}: {}", db_url, e),
            }
        }
        let dbconn = SqlitePool::connect(db_url.clone()).await.expect("");

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
