use crate::actor::Actor;
use crate::actor::ActorHandle;
use crate::message::ActorError;
use crate::message::Message;
use crate::message::Envelope;
use crate::nvtime::OffsetDateTimeWrapper;
use async_trait::async_trait;
use serde_json::from_str;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use time::OffsetDateTime;
use tokio::sync::mpsc;

pub struct StoreActor {
    pub receiver: mpsc::Receiver<Envelope>,
    pub dbconn: Option<sqlx::SqlitePool>,
    pub namespace: String,
    pub disable_duplicate_detection: bool,
}

#[async_trait]
impl Actor for StoreActor {
    async fn stop(&self) {
        if let Some(c) = &self.dbconn {
            c.close().await;
        }
    }
    async fn handle_envelope(&mut self, envelope: Envelope) {
        let Envelope {
            message,
            respond_to,
            stream_to,
            datetime: sequence,
            ..
        } = envelope;

        match message {
            Message::Update {
                path,
                datetime,
                values,
            } => {
                let dt = if self.disable_duplicate_detection {
                    sequence
                } else {
                    datetime
                };
                match self.insert_update(&path, dt, sequence, values).await {
                    Ok(_) => {
                        if let Some(respond_to) = respond_to {
                            match respond_to.send(Ok(Message::EndOfStream {})) {
                                Ok(_) => (),
                                Err(err) => {
                                    log::error!(
                                        "Cannot respond to 'ask' with confirmation: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(respond_to) = respond_to {
                            match respond_to.send(Err(ActorError {
                                reason: e.to_string(),
                            })) {
                                Ok(_) => (),
                                Err(err) => {
                                    log::error!(
                                        "Cannot respond to 'ask' with confirmation: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Message::LoadCmd { path } => {
                log::trace!("{path} load started...");
                if let Some(stream_to) = stream_to {
                    log::trace!("Handling LoadCmd for {}", path);
                    if let Some(dbconn) = &self.dbconn {
                        let rows: Vec<Message> = self.get_jrnl(dbconn, &path).await;
                        log::trace!(
                            "Handling LoadCmd jrnl for {} items count: {}",
                            path,
                            rows.len()
                        );
                        for message in rows {
                            match stream_to.send(message).await {
                                Ok(_) => (),
                                Err(err) => {
                                    log::error!("Can not send jrnl event from helper: {}", err);
                                }
                            }
                        }
                    }
                    match stream_to.send(Message::EndOfStream {}).await {
                        Ok(_) => (),
                        Err(err) => {
                            log::error!("Can not integrate from helper: {}", err);
                        }
                    }
                    stream_to.closed().await;
                }
            }
            m => log::warn!("Unexpected: {:?}", m),
        }
    }
}

impl StoreActor {
    /// actor private constructor
    fn new(
        receiver: mpsc::Receiver<Envelope>,
        dbconn: Option<sqlx::SqlitePool>,
        namespace: String,
        disable_duplicate_detection: bool,
    ) -> Self {
        Self {
            receiver,
            dbconn,
            namespace,
            disable_duplicate_detection,
        }
    }

    /// retrieve the time series of events (observations) for the actor that is being resurrected
    async fn get_jrnl(&self, dbconn: &SqlitePool, path: &str) -> Vec<Message> {
        let v = sqlx::query("SELECT timestamp, values_str FROM updates WHERE path = ?")
            .bind(path)
            .try_map(|row: sqlx::sqlite::SqliteRow| {
                //let date_parsed: OffsetDateTimeWrapper = from_str(row.get(0)).unwrap();
                let date_parsed_i64: i64 = from_str(row.get(0)).unwrap();
                let date_parsed: OffsetDateTimeWrapper = OffsetDateTimeWrapper {
                    datetime_i64: date_parsed_i64,
                };

                let values: HashMap<i32, f64> =
                    serde_json::from_str(row.try_get(1).expect("cannot extract values"))
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))
                        .expect("cannot return values");

                Ok(Message::Update {
                    path: String::from(path),
                    datetime: date_parsed.to_ts(),
                    values,
                })
            })
            .fetch_all(dbconn)
            .await
            .expect("cannot load from db");
        log::trace!(
            "fetched jrnl size {} for {}. last rec: {:?}",
            v.len(),
            path,
            v.last()
        );

        v
    }

    /// record the latest event in the actors state
    async fn insert_update(
        &self,
        path: &String,
        datetime: OffsetDateTime,
        sequence: OffsetDateTime,
        values: HashMap<i32, f64>,
    ) -> Result<(), sqlx::error::Error> {
        // store this is a db with the key as 'path'
        if let Some(dbconn) = &self.dbconn {
            let dt_wrapper = OffsetDateTimeWrapper::new(datetime);
            let sequence_wrapper = OffsetDateTimeWrapper::new(sequence);

            match sqlx::query(
                "INSERT INTO updates (path, timestamp, sequence, values_str) VALUES (?,?,?,?)",
            )
            .bind(path.clone())
            .bind(dt_wrapper.datetime_i64)
            .bind(sequence_wrapper.datetime_i64)
            .bind(serde_json::to_string(&values).expect("cannot serialize values"))
            .execute(dbconn)
            .await
            {
                Ok(_) => {
                    log::trace!("jrnled Update for {}", path);
                    Ok(())
                }
                Err(e) => {
                    log::warn!("jrnling for {} failed: {:?}", path, e);
                    Err(e)
                }
            }
        } else {
            log::error!("db conn not set");
            Ok(())
        }
    }
}

/// actor handle public constructor
#[must_use]
pub fn new(
    bufsz: usize,
    namespace: String,
    write_ahead_logging: bool,
    disable_duplicate_detection: bool,
) -> ActorHandle {
    async fn init_db(namespace: String, write_ahead_logging: bool) -> sqlx::SqlitePool {
        let db_url_string: String = format!("{namespace}.db");

        let db_url: &str = &db_url_string;

        let db_path = Path::new(db_url);

        if !db_path.exists() {
            match File::create(db_url) {
                Ok(_) => log::debug!("File {} has been created", db_url),
                Err(e) => log::warn!("Failed to create file {}: {}", db_url, e),
            }
        }

        let dbconn = SqlitePool::connect(db_url).await.expect("");

        if write_ahead_logging {
            sqlx::query("PRAGMA journal_mode = WAL;")
                .execute(&dbconn)
                .await
                .expect("");
        }

        // report on journal mode
        let rows = sqlx::query("PRAGMA journal_mode;")
            .fetch_all(&dbconn)
            .await
            .expect("can not check journal mode");
        let journal_mode: String = rows[0].get("journal_mode");
        log::info!("connected to db in journal_mode: {:?}", journal_mode);

        // Create table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS updates (
              path TEXT NOT NULL,
              timestamp TEXT NOT NULL,
              sequence TEXT NOT NULL,
              values_str TEXT NOT NULL,
              PRIMARY KEY (path, timestamp)
        )",
        )
        .execute(&dbconn)
        .await
        .expect("cannot create table");

        dbconn
    }

    async fn start(mut actor: StoreActor, namespace: String, write_ahead_logging: bool) {
        let dbconn = init_db(namespace, write_ahead_logging).await;

        actor.dbconn = Some(dbconn);

        while let Some(envelope) = actor.receiver.recv().await {
            actor.handle_envelope(envelope).await;
        }

        actor.stop().await;
    }

    let (sender, receiver) = mpsc::channel(bufsz);

    let actor = StoreActor::new(
        receiver,
        None,
        namespace.clone(),
        disable_duplicate_detection,
    );

    let actor_handle = ActorHandle::new(sender);

    tokio::spawn(start(actor, namespace, write_ahead_logging));

    actor_handle
}
