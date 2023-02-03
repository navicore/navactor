use clap::{Args, Parser, Subcommand};
use navactor::director;
use navactor::json_decoder;
use navactor::message::Message;
use navactor::message::Message::EndOfStream;
use navactor::stdin_actor;
use navactor::stdout_actor;
use navactor::store_actor_sqlite;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionVariant {
    On,
    Off,
}

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "nv is the CLI for the DtLaboratory project"
)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
struct Cli {
    #[arg(
        short,
        long,
        help = "Event store",
        long_help = "This file is the journal of all input.  Delete this file to cause the actors to calculate their state from only new observations."
    )]
    dbfile: Option<String>,
    #[arg(
        short,
        long,
        help = "Actor mailbox size",
        long_help = "The number of unread messages allowed in an actor's mailbox.  Small numbers can cause the system to single-thread / serialize work.  Large numbers can harm data integrity / commits and leave a lot of unfinished work if the server stops."
    )]
    buffer: Option<usize>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    #[arg(short, long, action = clap::ArgAction::SetTrue, help = "No output to console.", long_help = "Supress logging for slightly improved performance if you are loading a lot of piped data to a physical db file.")]
    silent: Option<bool>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "No on-disk db file", long_help = "For best performance, but you should not run with '--silent' as you won't know what the in-memory data was since it is now ephemeral.")]
    memory_only: Option<bool>,
    #[arg(short, long, action = clap::ArgAction::SetTrue, help = "Accept path+datetime collisions", long_help = "The journal stores and replays events in the order that they arrive but will ignore events that have a path and observation timestamp previously recorded - this is the best option for consistency and performance.  With 'disable-duplicate-detection' flag, the journal will accept observations regardless of the payload timestamp - this is good for testing and best for devices with unreliable notions of time.")]
    no_duplicate_detection: Option<bool>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "Write Ahead Logging", long_help = "Enable Write Ahead Logging (WAL) for performance improvements for use cases with frequent writes")]
    wal: Option<bool>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process incoming stream of internal formats into a namespace.  A
    /// namespace is an instance of GraphDirector.
    Update(Namespace),
    /// Get actor state for all actors in path
    Inspect(NvPath),
}

#[derive(Args)]
struct Namespace {
    namespace: String,
}

#[derive(Args)]
struct NvPath {
    /// actor path
    path: String,
}

fn update(
    namespace: Namespace,
    bufsz: usize,
    runtime: &Runtime,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    no_dupelicate_detection: OptionVariant,
) {
    let result = run_async_update(
        namespace,
        bufsz,
        silent,
        memory_only,
        write_ahead_logging,
        no_dupelicate_detection,
    );
    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            log::error!("can not launch thread: {e}");
        }
    }
}

async fn run_async_update(
    namespace: Namespace,
    bufsz: usize,
    silent: OptionVariant,
    memory_only: OptionVariant,
    write_ahead_logging: OptionVariant,
    no_duplicate_detection: OptionVariant,
) -> Result<(), String> {
    let output = match silent {
        OptionVariant::Off => Some(stdout_actor::new(bufsz)),
        OptionVariant::On => None,
    };

    let store_actor = match memory_only {
        OptionVariant::Off => Some(store_actor_sqlite::new(
            bufsz,
            namespace.namespace.clone(),
            write_ahead_logging == OptionVariant::On,
            no_duplicate_detection == OptionVariant::On,
        )),
        OptionVariant::On => None,
    };

    let director_w_persist = director::new(namespace.namespace, bufsz, output, store_actor);

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

fn inspect(path: NvPath, bufsz: usize, runtime: &Runtime) {
    let result = run_async_inspect(path, bufsz);

    match runtime.block_on(result) {
        Ok(_) => {}
        Err(e) => {
            log::error!("cannot launch thread: {e}");
        }
    }
}

async fn run_async_inspect(path: NvPath, bufsz: usize) -> Result<(), String> {
    let p = std::path::Path::new(&path.path);
    let ns = p
        .components()
        .find(|c| *c != std::path::Component::RootDir)
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("unk");
    log::trace!("inspect of ns {ns}");
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false, false); // print state

    let director = director::new(path.path.clone(), bufsz, None, Some(store_actor));

    match director.ask(Message::Query { path: path.path }).await {
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
    match output.ask(Message::EndOfStream {}).await {
        Ok(EndOfStream {}) => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

/// control logging of nv and various libs via `RUST_LOG` env var like so:
///`std::env::set_var("RUST_LOG`", "debug,sqlx=warn");
fn main() {
    env_logger::init();
    log::info!("nv started");

    let cli = Cli::parse();
    let bufsz: usize = cli.buffer.unwrap_or(8);
    let silent = cli.silent.map(|s| {
        if s {
            OptionVariant::On
        } else {
            OptionVariant::Off
        }
    });
    let memory_only = cli.memory_only.map(|m| {
        if m {
            OptionVariant::On
        } else {
            OptionVariant::Off
        }
    });
    let write_ahead_logging = cli.wal.map(|w| {
        if w {
            OptionVariant::On
        } else {
            OptionVariant::Off
        }
    });
    let no_dupelicate_detection = cli.no_duplicate_detection.map(|n| {
        if n {
            OptionVariant::On
        } else {
            OptionVariant::Off
        }
    });

    let runtime = Runtime::new().unwrap_or_else(|e| panic!("Error creating runtime: {e}"));

    match cli.command {
        Commands::Update(namespace) => update(
            namespace,
            bufsz,
            &runtime,
            silent.unwrap_or(OptionVariant::Off),
            memory_only.unwrap_or(OptionVariant::Off),
            write_ahead_logging.unwrap_or(OptionVariant::Off),
            no_dupelicate_detection.unwrap_or(OptionVariant::Off),
        ),
        Commands::Inspect(path) => inspect(path, bufsz, &runtime),
    }

    log::info!("nv stopped.");
}
