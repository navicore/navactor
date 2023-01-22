use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use sqlx::SqlitePool;
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
            respond_to: _,
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
                    log::debug!("jrnling Update for {}", path);
                    sqlx::query("INSERT INTO updates (path, timestamp, 'values') VALUES (?,?,?)")
                        .bind(path)
                        .bind(datetime.to_string())
                        .bind(serde_json::to_string(&values).expect(""))
                        .execute(dbconn)
                        .await
                        .expect("db insert failed");
                } else {
                    log::error!("db conn not set");
                }
            }
            Message::LoadCmd { path } => {
                log::debug!("handling LoadCmd for {}", path);
                if let Some(stream_to) = stream_to {
                    if let Some(m) = next_message {
                        stream_to
                            .send(m)
                            .await
                            .expect("can not integrate from helper");
                        stream_to
                            .send(Message::EndOfStream {})
                            .await
                            .expect("can not integrate from helper");
                    }
                }

                // TODO: handle LoadCmd messages
                // 1. open db with the path as key
                // 2. write events to "stream_to"
                // 3. write next_message to "stream_to"
                // 4. write EndOfStream to "stream_to"
                // 5. close send???
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
              'values' TEXT NOT NULL
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
    }

    let (sender, receiver) = mpsc::channel(bufsz);
    let actor = StoreActor::new(receiver, None);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
