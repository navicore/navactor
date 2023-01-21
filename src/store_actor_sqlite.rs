use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::Message;
use crate::message::MessageEnvelope;
use async_trait::async_trait;
use rusqlite::params;
use rusqlite::{Connection, Result};
use tokio::sync::mpsc;

/// TODO: needs a file name and path that relates to the namespace
pub struct StoreActor {
    pub receiver: mpsc::Receiver<MessageEnvelope>,
    pub dbconn: Connection,
}

#[async_trait]
impl Actor for StoreActor {
    /// for LoadCmd:
    ///   1. open db with the path as key
    ///   2. write events to "stream_to"
    ///   3. write next_message to "stream_to"
    ///   4. write EndOfStream to "stream_to"
    ///   5. closed send
    /// for Update:
    ///   1. open db
    ///   2. insert Update msg with key as path
    ///   3. closedb
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
            Message::Update { path, .. } => {
                log::debug!("jrnling Update for {}", path);
                // TODO: store this is a db with the key as 'path'
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
                        stream_to.closed().await;
                    }
                }
            }
            m => log::warn!("unexpected: {:?}", m),
        }
    }
}

/// actor private constructor
impl StoreActor {
    fn new(receiver: mpsc::Receiver<MessageEnvelope>, dbconn: Connection) -> Self {
        StoreActor { receiver, dbconn }
    }
}

/// actor handle public constructor
pub fn new(bufsz: usize) -> ActorHandle {
    async fn start(mut actor: StoreActor) {
        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }
    }
    let (sender, receiver) = mpsc::channel(bufsz);
    let dbconn = Connection::open("my_db.sqlite3").unwrap(); // TODO: safe?

    dbconn.execute("CREATE TABLE IF NOT EXISTS updates (path TEXT NOT NULL, timestamp TIMESTAMP NOT NULL, 'values' TEXT NOT NULL, PRIMARY KEY (path, timestamp))", params![])
        .expect("cannot create db");
    let actor = StoreActor::new(receiver, dbconn);
    let actor_handle = ActorHandle::new(sender);
    tokio::spawn(start(actor));
    actor_handle
}
