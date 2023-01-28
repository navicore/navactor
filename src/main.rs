use clap::{Args, Parser, Subcommand};
use navactor::director;
use navactor::json_decoder;
use navactor::message::Message;
use navactor::message::Message::EndOfStream;
use navactor::stdin_actor;
use navactor::stdout_actor;
use navactor::store_actor_sqlite;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "nv is the CLI for the DtLaboratory project"
)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long)]
    dbfile: Option<String>,
    #[arg(
        short,
        long,
        long_help = "The number of unread messages allowed in an actor's mailbox.  Small numbers can cause the system to single-thread / serialize work.  Large numbers can harm data integrity / commits and leave a lot of unfinished work if the server stops."
    )]
    buffer: Option<usize>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    #[arg(short, long, action = clap::ArgAction::SetTrue, help = "No output to console.", long_help = "Supress logging for slightly improved performance if you are loading a lot of piped data to a physical db file.")]
    silent: Option<bool>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "no on-disk db file", long_help = "For best performance, but you should not run with '--silent' as you won't know what the in-memory data was since it is now ephemeral.")]
    memory_only: Option<bool>,
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
    runtime: Runtime,
    silent: bool,
    memory_only: bool,
    write_ahead_logging: bool,
) {
    let result = run_async_update(namespace, bufsz, silent, memory_only, write_ahead_logging);
    runtime.block_on(result).expect("An error occurred")
}

async fn run_async_update(
    namespace: Namespace,
    bufsz: usize,
    silent: bool,
    memory_only: bool,
    write_ahead_logging: bool,
) -> Result<(), String> {
    let output = if silent {
        None
    } else {
        Some(stdout_actor::new(bufsz)) // print state changes
    };

    let store_actor = if memory_only {
        None
    } else {
        Some(store_actor_sqlite::new(
            bufsz,
            namespace.namespace.clone(),
            write_ahead_logging,
        ))
    };

    let director_w_persist = director::new(namespace.namespace, bufsz, output, store_actor);

    let json_decoder_actor = json_decoder::new(bufsz, director_w_persist); // parse input

    let input = stdin_actor::new(bufsz, json_decoder_actor); // read from stdin

    match input.ask(Message::ReadAllCmd {}).await {
        EndOfStream {} => {
            log::trace!("end of stream");
            Ok(())
        }
        e => {
            log::error!("{:?}", e);
            Err("END and response: sucks.".to_string())
        }
    }
}

fn inspect(path: NvPath, bufsz: usize, runtime: Runtime) {
    let result = run_async_inspect(path, bufsz);

    runtime.block_on(result).expect("An error occurred");
}

async fn run_async_inspect(path: NvPath, bufsz: usize) -> Result<(), String> {
    let p = std::path::Path::new(&path.path);
    let ns = p
        .components()
        .find(|c| c != &std::path::Component::RootDir)
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap();

    log::trace!("inspect of ns {}", ns);
    let output = stdout_actor::new(bufsz); // print state

    let store_actor = store_actor_sqlite::new(bufsz, String::from(ns), false); // print state

    let director = director::new(path.path.clone(), bufsz, None, Some(store_actor));

    let m = director.ask(Message::Query { path: path.path }).await;

    output.tell(m).await;

    // send complete to keep the job running long enough to print the above
    match output.ask(Message::EndOfStream {}).await {
        EndOfStream {} => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

/// control logging of nv and various libs via RUST_LOG env var like so:
///std::env::set_var("RUST_LOG", "debug,sqlx=warn");
fn main() {
    env_logger::init();
    log::info!("nv started");

    let cli = Cli::parse();
    let bufsz: usize = cli.buffer.unwrap_or(8);
    let silent: bool = cli.silent.unwrap_or(false);
    let memory_only: bool = cli.memory_only.unwrap_or(false);
    let write_ahead_logging: bool = cli.wal.unwrap_or(false);

    let runtime = Runtime::new().unwrap_or_else(|e| panic!("Error creating runtime: {}", e));

    match cli.command {
        Commands::Update(namespace) => update(
            namespace,
            bufsz,
            runtime,
            silent,
            memory_only,
            write_ahead_logging,
        ),
        Commands::Inspect(path) => inspect(path, bufsz, runtime),
    }

    log::info!("nv stopped.");
}
