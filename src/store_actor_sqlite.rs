//!This module contains the implementation of the `StoreActor`, which is responsible for storing
//!all the state changes of an actor into a `SQLite` database.
//!
//!The module provides an API that is called by instances of the Actor trait when read and write
//!requests arrive. `StoreActor` is designed to store only a single file for storage so all reading
//!and writing must be done by messaging an instance of this actor type.
//!
//!The module also defines `StoreError`, a custom error type used for error handling, and
//!`StreamOption`, an enumeration type to define the behavior of temporary message streams.
//!
//!The `StoreActor` works with the `SqlitePool` object and the `sqlx` crate for interacting with
//!the database.
//!
//!This module also provides methods to retrieve the time series of events for the actor being
//!resurrected, define tables, enable write-ahead-logging mode for append-only-style db and
//!initialize the DB if it does not exist.
//!
//!The module is constructed as an actor handle that is expected to be used with the director
//!module in creating a new actor system.

use crate::actor::respond_or_log_error;
use crate::actor::Actor;
use crate::actor::Handle;
use crate::message::Envelope;
use crate::message::Message;
use crate::message::MtHint;
use crate::message::NvError;
use crate::message::NvResult;
use crate::nvtime::OffsetDateTimeWrapper;
use async_trait::async_trait;
use serde_json::from_str;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::path::Path;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::sync::oneshot::Sender;

// used by instances of the actor trait - TODO should this be here if no actor ifc uses it?
pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone)]
pub struct StoreError {
    pub reason: String,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bad actor state: {}", self.reason)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StreamOption {
    Close,
    LeaveOpen,
}

/// main persistence API - the navactor must have only a single file for
/// storage so all reading and writing must be done by messaging an instance
/// of this actor type
pub struct StoreActor {
    pub receiver: mpsc::Receiver<Envelope<f64>>,
    pub dbconn: Option<SqlitePool>,
    pub namespace: String,
    pub disable_duplicate_detection: bool,
}

async fn insert_gene_mapping(
    dbconn: &SqlitePool,
    path: &String,
    text: &String,
) -> Result<(), sqlx::error::Error> {
    match sqlx::query("INSERT INTO gene_mappings (path, text) VALUES (?,?)")
        .bind(path)
        .bind(text)
        .execute(dbconn)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            log::warn!("persisting gene mapping for {} failed: {:?}", path, e);
            Err(e)
        }
    }
}

/// record the latest event in the actors state
async fn insert_update(
    dbconn: &SqlitePool,
    path: &String,
    datetime: OffsetDateTime,
    sequence: OffsetDateTime,
    values: HashMap<i32, f64>,
) -> Result<(), sqlx::error::Error> {
    // store this is a db with the key as 'path'
    let dt_wrapper = OffsetDateTimeWrapper::new(datetime);
    let sequence_wrapper = OffsetDateTimeWrapper::new(sequence);

    match sqlx::query(
        "INSERT INTO updates (path, timestamp, sequence, values_str) VALUES (?,?,?,?)",
    )
    .bind(path.clone())
    .bind(dt_wrapper.datetime_num)
    .bind(sequence_wrapper.datetime_num)
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
}

/// retrieve the time series of events (observations) for the actor that is being resurrected
async fn get_jrnl(dbconn: &SqlitePool, path: &str) -> StoreResult<Vec<Message<f64>>> {
    match get_values(path, dbconn).await {
        Ok(v) => Ok(v),
        Err(e) => {
            log::error!("cannot load update jrnl from db: {e:?}");
            Err(StoreError {
                reason: format!("cannot load jrnl from db: {e:?}"),
            })
        }
    }
}

/// retrieve the time series of events (observations) for the actor that is being resurrected
async fn get_mappings(dbconn: &SqlitePool, path: &str) -> StoreResult<Vec<Message<f64>>> {
    match get_mappings_for_ns(path, dbconn).await {
        Ok(v) => Ok(v),
        Err(e) => {
            log::error!("cannot load mappings from db: {e:?}");
            Err(StoreError {
                reason: format!("cannot load from db: {e:?}"),
            })
        }
    }
}

/// internal actor-to-actor communication outside of input-to-state_actor is
/// done with temporary streams (for now) and these streams are setup by
/// an orchestrator (usually director).
async fn stream_message(
    stream_to: &Option<mpsc::Sender<Message<f64>>>,
    message: Message<f64>,
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
    } else {
        log::trace!("no stream available for {message}");
    }
}

async fn handle_gene_mapping(
    path: String,
    text: String,
    dbconn: &SqlitePool,
    respond_to: Option<Sender<NvResult<Message<f64>>>>,
) {
    match insert_gene_mapping(dbconn, &path, &text).await {
        Ok(_) => {
            log::debug!("gene_mapping '{path}' -> '{text}' persisted");
            respond_or_log_error(respond_to, Ok(Message::EndOfStream {}));
        }
        Err(e) => respond_or_log_error(
            respond_to,
            Err(NvError {
                reason: e.to_string(),
            }),
        ),
    }
}

async fn handle_update(
    path: String,
    datetime: OffsetDateTime,
    sequence: OffsetDateTime,
    values: HashMap<i32, f64>,
    disable_duplicate_detection: bool,
    dbconn: &SqlitePool,
    respond_to: Option<Sender<NvResult<Message<f64>>>>,
) {
    // sequence should be the envelope dt and should never cause a collision
    let dt = if disable_duplicate_detection {
        sequence
    } else {
        datetime
    };
    match insert_update(dbconn, &path, dt, sequence, values).await {
        Ok(_) => respond_or_log_error(respond_to, Ok(Message::EndOfStream {})),
        Err(e) => respond_or_log_error(
            respond_to,
            Err(NvError {
                reason: e.to_string(),
            }),
        ),
    }
}

/// a load command is indicates a new actor is expecting its journal.  the
/// message contains a `stream_to` - read each row from the DB and write
/// a message for each row to the actor at the other end of the `stream_to`
/// connection.  after the last row, write an `EndOfStream` msg and close the
/// connection
async fn handle_load_cmd(
    path: String,
    dbconn: &SqlitePool,
    stream_to: Option<mpsc::Sender<Message<f64>>>,
) {
    match get_jrnl(dbconn, &path).await {
        Ok(rows) => {
            for message in rows {
                stream_message(&stream_to, message, StreamOption::LeaveOpen).await;
            }
        }
        Err(e) => {
            log::error!("cannot load jrnl: {path} {e:?}");
        }
    };
    stream_message(&stream_to, Message::EndOfStream {}, StreamOption::Close).await;
}

async fn handle_gene_mapping_load_cmd(
    path: String,
    dbconn: &SqlitePool,
    stream_to: Option<mpsc::Sender<Message<f64>>>,
) {
    match get_mappings(dbconn, &path).await {
        Ok(rows) => {
            for message in rows {
                stream_message(&stream_to, message, StreamOption::LeaveOpen).await;
            }
        }
        Err(e) => {
            log::error!("cannot load gene mapping jrnl: {path} {e:?}");
        }
    };
    stream_message(&stream_to, Message::EndOfStream {}, StreamOption::Close).await;
}

#[async_trait]
impl Actor for StoreActor {
    /// the main entry point to every actor - this is where the jrnl read and
    /// write requests arrive
    async fn handle_envelope(&mut self, envelope: Envelope<f64>) {
        if let Some(dbconn) = &self.dbconn {
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
                    handle_update(
                        path,
                        datetime,
                        sequence,
                        values,
                        self.disable_duplicate_detection,
                        dbconn,
                        respond_to,
                    )
                    .await;
                }
                Message::LoadCmd { path, hint } if hint == MtHint::GeneMapping => {
                    handle_gene_mapping_load_cmd(path, dbconn, stream_to).await;
                }
                Message::LoadCmd { path, hint } if hint == MtHint::Update => {
                    handle_load_cmd(path, dbconn, stream_to).await;
                }
                Message::Content { path, text, hint }
                    if path.is_some() && hint == MtHint::GeneMapping =>
                {
                    match path {
                        Some(path) => {
                            handle_gene_mapping(path, text, dbconn, respond_to).await;
                        }
                        _ => {
                            log::error!("path not set");
                        }
                    }
                }
                m => log::warn!("Unexpected: {m}"),
            }
        } else {
            log::error!("DB not configured");
        }
    }
    async fn start(&mut self) {}
    async fn stop(&self) {
        if let Some(c) = &self.dbconn {
            c.close().await;
        }
    }
}

// TODO: store mappings with namespace / path compound key
async fn get_mappings_for_ns(
    path: &str,
    dbconn: &SqlitePool,
) -> Result<Vec<Message<f64>>, sqlx::error::Error> {
    log::debug!("loading mappings for path {path}");
    sqlx::query("SELECT path, text FROM gene_mappings;")
        .bind(path)
        .try_map(|row: sqlx::sqlite::SqliteRow| {
            let path = match row.try_get(0) {
                //let path = match from_str(row.get(0)) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("cannot read path");
                    return Err(sqlx::Error::Decode(Box::new(e)));
                }
            };

            let text = match row.try_get(1) {
                //let text = match from_str(row.get(1)) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("cannot read text");
                    return Err(sqlx::Error::Decode(Box::new(e)));
                }
            };

            Ok(Message::Content {
                path: Some(path),
                text,
                hint: MtHint::GeneMapping,
            })
        })
        .fetch_all(dbconn)
        .await
}

async fn get_values(
    path: &str,
    dbconn: &SqlitePool,
) -> Result<Vec<Message<f64>>, sqlx::error::Error> {
    sqlx::query("SELECT timestamp, values_str FROM updates WHERE path = ?")
        .bind(path)
        .try_map(|row: sqlx::sqlite::SqliteRow| {
            let date_parsed_num = match from_str(row.get(0)) {
                Ok(val) => val,
                Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
            };

            let date_parsed = OffsetDateTimeWrapper {
                datetime_num: date_parsed_num,
            };

            let values = match row.try_get(1) {
                Ok(val_str) => match from_str(val_str) {
                    Ok(val) => val,
                    Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
                },
                Err(e) => return Err(sqlx::Error::Decode(Box::new(e))),
            };

            let dt = match date_parsed.to_ts() {
                Ok(dt) => dt,
                Err(e) => {
                    log::error!("can not parse date - using 'now': {e}");
                    OffsetDateTime::now_utc()
                }
            };
            Ok(Message::Update {
                path: String::from(path),
                datetime: dt,
                values,
            })
        })
        .fetch_all(dbconn)
        .await
}

impl StoreActor {
    /// actor private constructor
    const fn new(
        receiver: mpsc::Receiver<Envelope<f64>>,
        dbconn: Option<SqlitePool>,
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
}

async fn define_gene_mapping_table_if_not_exist(
    db_url: &str,
    dbconn: &SqlitePool,
) -> StoreResult<()> {
    let rows = sqlx::query("PRAGMA journal_mode;")
        .fetch_all(dbconn)
        .await
        .map_err(|e| StoreError {
            reason: format!("Failed to fetch journal_mode: {e}"),
        })?;

    let journal_mode: String = rows[0].get("journal_mode");
    log::info!("connected to db in journal_mode for mappings: {journal_mode}");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS gene_mappings (
              path TEXT NOT NULL,
              text TEXT NOT NULL,
              PRIMARY KEY (path)
        )",
    )
    .execute(dbconn)
    .await
    .map_err(|e| StoreError {
        reason: format!("Failed to create file {db_url}: {e}"),
    })?;

    Ok(())
}

/// define table if it does not exist and log to console the journal mode
async fn define_updates_table_if_not_exist(db_url: &str, dbconn: &SqlitePool) -> StoreResult<()> {
    let rows = sqlx::query("PRAGMA journal_mode;")
        .fetch_all(dbconn)
        .await
        .map_err(|e| StoreError {
            reason: format!("Failed to fetch journal_mode: {e}"),
        })?;

    let journal_mode: String = rows[0].get("journal_mode");
    log::info!("connected to db in journal_mode: {journal_mode}");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS updates (
              path TEXT NOT NULL,
              timestamp TEXT NOT NULL,
              sequence TEXT NOT NULL,
              values_str TEXT NOT NULL,
              PRIMARY KEY (path, timestamp)
        )",
    )
    .execute(dbconn)
    .await
    .map_err(|e| StoreError {
        reason: format!("Failed to create file {db_url}: {e}"),
    })?;

    Ok(())
}

/// enable write-ahead-logging mode for append-only-style db
async fn enable_wal(db_url: &str, dbconn: &SqlitePool) -> StoreResult<()> {
    match sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(dbconn)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(StoreError {
            reason: format!("Failed to create file {db_url}: {e}"),
        }),
    }
}

/// multiple operations:
/// 1. initialize the DB if it does not exist
/// 2. connect
/// 3. configure wal
/// 4  report to console
/// 5. return a db connection object.
async fn init_db(namespace: String, write_ahead_logging: bool) -> StoreResult<SqlitePool> {
    let db_url_string: String = format!("{namespace}.db");
    let db_url: &str = &db_url_string;
    let db_path = Path::new(db_url);
    if !db_path.exists() {
        match File::create(db_url) {
            Ok(_) => log::debug!("File {} has been created", db_url),
            Err(e) => {
                return Err(StoreError {
                    reason: format!("Failed to create file {db_url}: {e}"),
                });
            }
        }
    }

    // connect to db, enable wal if configured, and report to the console on
    // how the db is configured
    match SqlitePool::connect(db_url).await {
        Ok(dbconn) => {
            if write_ahead_logging {
                match enable_wal(db_url, &dbconn).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            match define_updates_table_if_not_exist(db_url, &dbconn).await {
                Ok(_) => match define_gene_mapping_table_if_not_exist(db_url, &dbconn).await {
                    Ok(_) => Ok(dbconn),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            log::error!("cannot connect to db: {e:?}");
            Err(StoreError {
                reason: format!("{e:?}"),
            })
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
    async fn start(mut actor: StoreActor, namespace: String, write_ahead_logging: bool) {
        // create a db connection and put it in the actor state
        // the connection is made after spawning the new thread which is why
        // the db connection is not passed to the actor constructor
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
