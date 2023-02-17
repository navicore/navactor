use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator};
use navactor::cli;
use navactor::cli::Commands;
use navactor::director;
use navactor::json_decoder;
use navactor::message::Message;
use navactor::message::Message::EndOfStream;
use navactor::stdin_actor;
use navactor::stdout_actor;
use navactor::store_actor_sqlite;
use std::io;
use tokio::runtime::Runtime;

fn update(
    namespace: String,
    bufsz: usize,
    runtime: &Runtime,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    allow_dupelicates: OptionVariant,
) {
    let result = run_async_update(
        namespace,
        bufsz,
        silent,
        memory_only,
        write_ahead_logging,
        allow_dupelicates,
    );
    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            log::error!("can not launch thread: {e}");
        }
    }
}

async fn run_async_update(
    namespace: String,
    bufsz: usize,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    allow_duplicates: OptionVariant,
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
            allow_duplicates == OptionVariant::On,
        )),
        OptionVariant::On => None,
    };

    let director_w_persist = director::new(&namespace, bufsz, output, store_actor);

    let json_decoder_actor = json_decoder::new(bufsz, director_w_persist);

    let input = stdin_actor::new(bufsz, json_decoder_actor);

    match input.ask(Message::ReadAllCmd {}).await {
        Ok(EndOfStream {}) => {
            log::trace!("end of stream");
            Ok(())
        }
        e => {
            log::error!("{:?}", e);
            Err("END and response: sucks.".to_string())
        }
    }
}

fn inspect(path: String, bufsz: usize, runtime: &Runtime) {
    let result = run_async_inspect(path, bufsz);

    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            log::error!("cannot launch thread: {e}");
        }
    }
}

fn configure(path: &String, gene: &String, _: &Runtime) {
    log::error!("not implemented. path: {path} gene: {gene}");
}

async fn run_async_inspect(path: String, bufsz: usize) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let ns = p
        .components()
        .find(|c| *c != std::path::Component::RootDir)
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("unk");
    log::trace!("inspect of ns {ns}");
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false, false); // print state

    let director = director::new(&path.clone(), bufsz, None, Some(store_actor));

    match director.ask(Message::Query { path }).await {
        Ok(m) => match output.tell(m).await {
            Ok(_) => {}
            Err(e) => {
                log::warn!("cannot tell {e}");
            }
        },
        Err(e) => {
            log::error!("error {e}");
        }
    }

    // send complete to keep the job running long enough to print the above
    match output.ask(EndOfStream {}).await {
        Ok(EndOfStream {}) => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionVariant {
    On,
    Off,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

/// control logging of nv and various libs via `RUST_LOG` env var like so:
///`std::env::set_var("RUST_LOG`", "debug,sqlx=warn");
fn main() {
    env_logger::init();
    log::info!("nv started");

    let pcli = cli::Cli::parse();
    let bufsz: usize = pcli.buffer.unwrap_or(8);
    let memory_only = pcli.memory_only.map(|m| {
        if m {
            OptionVariant::On
        } else {
            OptionVariant::Off
        }
    });

    let runtime = Runtime::new().unwrap_or_else(|e| panic!("Error creating runtime: {e}"));

    match pcli.command {
        Commands::Update {
            namespace,
            silent,
            wal,
            allow_duplicates,
        } => {
            let namespace = namespace.unwrap_or_else(|| "actors".to_owned());
            let silent = match silent {
                Some(true) => OptionVariant::On,
                _ => OptionVariant::Off,
            };
            let memory_only = memory_only.unwrap_or(OptionVariant::Off);
            let wal = match wal {
                Some(true) => OptionVariant::On,
                _ => OptionVariant::Off,
            };
            let allow_duplicates = match allow_duplicates {
                Some(true) => OptionVariant::On,
                _ => OptionVariant::Off,
            };
            update(
                namespace,
                bufsz,
                &runtime,
                silent,
                memory_only,
                wal,
                allow_duplicates,
            );
        }
        Commands::Inspect { path } => inspect(path, bufsz, &runtime),
        Commands::Configure { path, gene } => configure(&path, &gene, &runtime),
        Commands::Completions { shell } => {
            let mut cmd = cli::Cli::command();
            print_completions(shell, &mut cmd);
        }
    }

    log::info!("nv stopped.");
}
