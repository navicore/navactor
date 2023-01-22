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
}

#[async_trait]
impl Actor for StoreActor {
    async fn handle_envelope(&mut self, envelope: MessageEnvelope) {
        let MessageEnvelope {
            message,
            respond_to,
            datetime: _,
            stream_to,
            stream_from: _,
            next_message,
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
                    log::debug!("jrnling Update for {}...", path);

                    let dt_wrapper = OffsetDateTimeWrapper { datetime };
                    let datetime_str = to_string(&dt_wrapper).unwrap();

                    sqlx::query("INSERT INTO updates (path, timestamp, values_str) VALUES (?,?,?)")
                        .bind(path.clone())
                        .bind(datetime_str)
                        .bind(serde_json::to_string(&values).expect("cannot serialize values"))
                        .execute(dbconn)
                        .await
                        .expect("db insert failed");
                    log::debug!("...jrnled Update for {}", path);
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
                    log::debug!("handling LoadCmd for {}", path);
                    // 1. query db with the path as key
                    // 2. write events to "stream_to"
                    // 3. write next_message to "stream_to"
                    // 4. write EndOfStream to "stream_to"
                    // 5. close stream_to
                    if let Some(dbconn) = &self.dbconn {
                        log::debug!("handling LoadCmd jrnl retrieval for {}", path);
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
                        for message in rows {
                            log::debug!("handling LoadCmd jrnl item send for {}", path);
                            stream_to
                                .send(message)
                                .await
                                .expect("can not send jrnl event from helper");
                        }
                    }

                    if let Some(m) = next_message {
                        log::debug!("handling LoadCmd next_message send for {}", path);
                        stream_to
                            .send(m)
                            .await
                            .expect("can not integrate from helper");
                        log::debug!("handling LoadCmd EndOfStream send for {}", path);
                        stream_to
                            .send(Message::EndOfStream {})
                            .await
                            .expect("can not integrate from helper");
                    }
                    stream_to.closed().await;
                }
            }
            m => log::warn!("unexpected: {:?}", m),
        }
    }
}

/// actor private constructor
impl StoreActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, dbconn: Option<sqlx::SqlitePool>) -> Self {
        StoreActor { receiver, dbconn }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize) -> ActorHandle {
    async fn init_db() -> sqlx::SqlitePool {
        // TODO: default is memory but file comes from nv cli
        let db_url = "navactor.db";
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

    async fn start(mut actor: StoreActor) {
        let dbconn = init_db().await;
        // TODO: dbcon seems to be None
        // TODO: dbcon seems to be None
        // TODO: dbcon seems to be None
        // TODO: dbcon seems to be None
        // TODO: dbcon seems to be None
        // TODO: dbcon seems to be None
        actor.dbconn = Some(dbconn);
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
        // let sigint = signal(SignalKind::interrupt());
        // let sigterm = signal(SignalKind::terminate());
        // let sigint = signal(SignalKind::interrupt()).await;
        // let sigterm = signal(SignalKind::terminate()).await;
        // join!(sigint, sigterm);
        // select! {
        //     _ = sigint => println!("Ctrl+C received"),
        //     _ = sigterm => println!("SIGTERM received"),
        // }
        //dbconn.close().await;
    }

    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StoreActor::new(receiver, None);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
