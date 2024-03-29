use crate::actors::actor::Handle;
use crate::actors::director;
use crate::actors::genes::gene::GeneType;
use crate::actors::message::Message;
use crate::actors::message::Message::EndOfStream;
use crate::actors::message::MtHint;
use crate::actors::store_actor_sqlite;
use crate::io::json_decoder;
use crate::io::net::api_server::serve;
use crate::io::net::api_server::HttpServerConfig;
use crate::io::stdin_actor;
use crate::io::stdout_actor;
use clap::Command;
use clap_complete::{generate, Generator};
use std::io;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::error;
use tracing::trace;
use tracing::warn;

pub fn run_serve(
    server_config: HttpServerConfig,
    runtime: &Runtime,
    uipath: Option<String>,
    disable_ui: Option<bool>,
    write_ahead_logging: OptionVariant,
    disable_dupe_detection: OptionVariant,
) {
    let result = run_async_serve(
        server_config,
        uipath,
        disable_ui,
        write_ahead_logging,
        disable_dupe_detection,
    );
    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            error!("can not launch server: {e}");
        }
    }
}

fn setup_server_actor(
    db_file_prefix: String,
    namespace: &str,
    write_ahead_logging: OptionVariant,
    disable_dupe_detection: OptionVariant,
) -> Arc<Handle> {
    let store_actor: Handle = store_actor_sqlite::new(
        8,
        db_file_prefix,
        write_ahead_logging == OptionVariant::On,
        disable_dupe_detection == OptionVariant::On,
    );

    let director_with_persistence = director::new(namespace, 8, None, Some(store_actor));

    Arc::new(director_with_persistence)
}

async fn run_async_serve(
    server_config: HttpServerConfig,
    uipath: Option<String>,
    disable_ui: Option<bool>,
    write_ahead_logging: OptionVariant,
    disable_dupe_detection: OptionVariant,
) -> Result<(), String> {
    let shared_handle: Arc<Handle> = setup_server_actor(
        server_config.namespace.clone(),
        server_config.namespace.as_str(),
        write_ahead_logging,
        disable_dupe_detection,
    );
    match serve(shared_handle, server_config, uipath, disable_ui).await {
        Ok(()) => Ok(()),
        e => {
            error!("{e:?}");
            Err(format!("{e:?}"))
        }
    }
}

pub fn update(
    namespace: String,
    bufsz: usize,
    runtime: &Runtime,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    disable_dupe_detection: OptionVariant,
) {
    let result = run_async_update(
        namespace,
        bufsz,
        silent,
        memory_only,
        write_ahead_logging,
        disable_dupe_detection,
    );
    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            error!("can not launch thread: {e}");
        }
    }
}

async fn run_async_update(
    namespace: String,
    bufsz: usize,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    disable_dupe_detection: OptionVariant,
) -> Result<(), String> {
    let output = match silent {
        OptionVariant::Off => Some(stdout_actor::new(bufsz)),
        OptionVariant::On => None,
    };

    let store_actor = match memory_only {
        OptionVariant::Off => Some(store_actor_sqlite::new(
            bufsz,
            namespace.clone(),
            write_ahead_logging == OptionVariant::On,
            disable_dupe_detection == OptionVariant::On,
        )),
        OptionVariant::On => None,
    };

    let director_w_persist = director::new(namespace.as_str(), bufsz, output, store_actor);

    let json_decoder_actor = json_decoder::new(bufsz, director_w_persist);

    let input = stdin_actor::new(bufsz, json_decoder_actor);

    match input.ask(Message::ReadAllCmd {}).await {
        Ok(EndOfStream {}) => {
            trace!("end of stream");
            Ok(())
        }
        e => {
            error!("{:?}", e);
            Err("END and response: sucks.".to_string())
        }
    }
}

pub fn configure(path: String, gene_type: GeneType, bufsz: usize, runtime: &Runtime) {
    let result = run_async_configure(path, gene_type, bufsz);

    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            error!("cannot launch thread: {e}");
        }
    }
}

async fn run_async_configure(
    path: String,
    gene_type: GeneType,
    bufsz: usize,
) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let ns = p
        .components()
        .find(|c| *c != std::path::Component::RootDir)
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("unk");
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false, false); // print state

    let director = director::new(path.as_str(), bufsz, None, Some(store_actor));

    let gene_type_str = match gene_type {
        GeneType::Accum => "accum",
        GeneType::Gauge => "gauge",
        _ => "gauge_and_accum",
    };

    match director
        .ask(Message::Content {
            path: Some(path),
            text: String::from(gene_type_str),
            hint: MtHint::GeneMapping,
        })
        .await
    {
        Ok(m) => match output.tell(m).await {
            Ok(_) => {}
            Err(e) => {
                warn!("cannot tell {e}");
            }
        },
        Err(e) => {
            error!("error {e}");
        }
    }

    // send complete to keep the job running long enough to print the above
    match output.ask(EndOfStream {}).await {
        Ok(EndOfStream {}) => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

pub fn explain(path: String, bufsz: usize, runtime: &Runtime) {
    let result = run_async_explain(path, bufsz);

    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            error!("cannot launch thread: {e}");
        }
    }
}

async fn run_async_explain(path: String, bufsz: usize) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let ns = p
        .components()
        .find(|c| *c != std::path::Component::RootDir)
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("unk");
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false, false); // print state

    let director = director::new(path.as_str(), bufsz, None, Some(store_actor));

    match director
        .ask(Message::Query {
            path,
            hint: MtHint::GeneMapping,
        })
        .await
    {
        Ok(m) => match output.tell(m).await {
            Ok(_) => {}
            Err(e) => {
                warn!("cannot tell {e}");
            }
        },
        Err(e) => {
            error!("error {e}");
        }
    }

    // send complete to keep the job running long enough to print the above
    match output.ask(EndOfStream {}).await {
        Ok(EndOfStream {}) => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

pub fn inspect(path: String, bufsz: usize, runtime: &Runtime) {
    let result = run_async_inspect(path, bufsz);

    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            error!("cannot launch thread: {e}");
        }
    }
}

async fn run_async_inspect(path: String, bufsz: usize) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let ns = p
        .components()
        .find(|c| *c != std::path::Component::RootDir)
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("unk");
    trace!("inspect of ns {ns}");
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false, false); // print state

    let director = director::new(path.as_str(), bufsz, None, Some(store_actor));

    match director
        .ask(Message::Query {
            path,
            hint: MtHint::State,
        })
        .await
    {
        Ok(m) => match output.tell(m).await {
            Ok(_) => {}
            Err(e) => {
                warn!("cannot tell {e}");
            }
        },
        Err(e) => {
            error!("error {e}");
        }
    }

    // send complete to keep the job running long enough to print the above
    match output.ask(EndOfStream {}).await {
        Ok(EndOfStream {}) => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionVariant {
    On,
    Off,
}

pub fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
