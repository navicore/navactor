use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::ActorError;
use crate::message::ActorResult;
use crate::message::Envelope;
use crate::message::Message;
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum StreamOption {
    Close,
    LeaveOpen,
}

pub struct StoreActor {
    pub receiver: mpsc::Receiver<Envelope>,
    pub dbconn: Option<sqlx::SqlitePool>,
    pub namespace: String,
    pub disable_duplicate_detection: bool,
}

async fn stream_message(
    stream_to: &Option<tokio::sync::mpsc::Sender<Message>>,
    message: Message,
    stream_option: StreamOption,
) {
    if let Some(stream_to) = stream_to {
        match stream_to.send(message).await {
            Ok(_) => (),
            Err(err) => {
                log::error!("Can not integrate from helper: {}", err);
            }
        }
        if stream_option == StreamOption::Close {
            stream_to.closed().await;
        };
    }
}

fn reply_or_log_error(
    respond_to: Option<tokio::sync::oneshot::Sender<ActorResult<Message>>>,
    result: ActorResult<Message>,
) {
    {
        if let Some(respond_to) = respond_to {
            match respond_to.send(result) {
                Ok(_) => (),
                Err(err) => {
                    log::error!("Cannot respond to 'ask' with confirmation: {:?}", err);
                }
            }
        }
    }
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
                    Ok(_) => reply_or_log_error(respond_to, Ok(Message::EndOfStream {})),
                    Err(e) => reply_or_log_error(
                        respond_to,
                        Err(ActorError {
                            reason: e.to_string(),
                        }),
                    ),
                }
            }

            Message::LoadCmd { path } => {
                if let Some(dbconn) = &self.dbconn {
                    match self.get_jrnl(dbconn, &path).await {
                        Ok(rows) => {
                            for message in rows {
                                stream_message(&stream_to, message, StreamOption::LeaveOpen).await;
                            }
                        }
                        Err(e) => {
                            log::error!("cannot load jrnl: {path} {e:?}");
                        }
                    };
                } else {
                    log::debug!("bypass journal for {path} - db not configured");
                }
                stream_message(&stream_to, Message::EndOfStream {}, StreamOption::Close).await;
            }
            m => log::warn!("Unexpected: {:?}", m),
        }
    }
}

impl StoreActor {
    /// actor private constructor
    const fn new(
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

    async fn get_jrnl(&self, dbconn: &SqlitePool, path: &str) -> ActorResult<Vec<Message>> {
        let v = sqlx::query("SELECT timestamp, values_str FROM updates WHERE path = ?")
            .bind(path)
            .try_map(|row: sqlx::sqlite::SqliteRow| {
                let date_parsed_i64 = match from_str(row.get(0)) {
                    Ok(val) => val,
                    Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
                };

                let date_parsed = OffsetDateTimeWrapper {
                    datetime_i64: date_parsed_i64,
                };

                let values = match row.try_get(1) {
                    Ok(val_str) => match serde_json::from_str(val_str) {
                        Ok(val) => val,
                        Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
                    },
                    Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
                };

                Ok(Message::Update {
                    path: String::from(path),
                    datetime: date_parsed.to_ts(),
                    values,
                })
            })
            .fetch_all(dbconn)
            .await;

        match v {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!("cannot load from db: {:?}", e);
                Err(ActorError {
                    reason: format!("cannot load from db: {e:?}"),
                })
            }
        }
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
            .bind(
                serde_json::to_string(&values)
                    .map_err(|e| {
                        log::error!("cannot serialize values: {e:?}");
                    })
                    .ok(),
            )
            .execute(dbconn)
            .await
            {
                Ok(_) => Ok(()),
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
) -> Handle {
    async fn init_db(
        namespace: String,
        write_ahead_logging: bool,
    ) -> ActorResult<sqlx::SqlitePool> {
        let db_url_string: String = format!("{namespace}.db");

        let db_url: &str = &db_url_string;

        let db_path = Path::new(db_url);

        if !db_path.exists() {
            match File::create(db_url) {
                Ok(_) => log::debug!("File {} has been created", db_url),
                Err(e) => {
                    return Err(ActorError {
                        reason: format!("Failed to create file {db_url}: {e}"),
                    });
                }
            }
        }

        match SqlitePool::connect(db_url).await {
            Ok(dbconn) => {
                if write_ahead_logging {
                    match sqlx::query("PRAGMA journal_mode = WAL;")
                        .execute(&dbconn)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(ActorError {
                                reason: format!("Failed to create file {db_url}: {e}"),
                            });
                        }
                    }
                }

                // report on journal mode
                match sqlx::query("PRAGMA journal_mode;").fetch_all(&dbconn).await {
                    Ok(rows) => {
                        let journal_mode: String = rows[0].get("journal_mode");
                        log::info!("connected to db in journal_mode: {:?}", journal_mode);

                        // Create table if it doesn't exist
                        match sqlx::query(
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
                        {
                            Ok(_) => Ok(dbconn),
                            Err(e) => {
                                return Err(ActorError {
                                    reason: format!("Failed to create file {db_url}: {e}"),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Err(ActorError {
                            reason: format!("Failed to create file {db_url}: {e}"),
                        });
                    }
                }
            }
            Err(e) => {
                log::error!("cannot connect to db: {e:?}");
                return Err(ActorError {
                    reason: format!("{e:?}"),
                });
            }
        }
    }

    async fn start(mut actor: StoreActor, namespace: String, write_ahead_logging: bool) {
        let dbconn = init_db(namespace, write_ahead_logging)
            .await
            .map_err(|e| {
                log::error!("cannot get dbconn: {e:?}");
            })
            .ok();

        actor.dbconn = dbconn;

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

    let actor_handle = Handle::new(sender);

    tokio::spawn(start(actor, namespace, write_ahead_logging));

    actor_handle
}
