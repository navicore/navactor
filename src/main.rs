use clap::{Args, Parser, Subcommand};
use log::debug;
use nv::director_w_sqlite;
use nv::json_decoder;
use nv::message::Message;
use nv::message::Message::IsCompleteMsg;
use nv::stdin_actor;
use nv::stdout_actor;
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
    #[arg(short, long)]
    buffer: Option<usize>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
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
struct NoArg {}

#[derive(Args)]
struct NvPath {
    /// actor path
    path: String,
}

#[derive(Args)]
struct Extractor {
    /// name of extractor
    extractor: String,
}

fn update(namespace: Namespace, bufsz: usize, runtime: Runtime) {
    let result = run_async_update(namespace, bufsz);
    runtime.block_on(result).expect("An error occurred");
}

async fn run_async_update(namespace: Namespace, bufsz: usize) -> Result<(), String> {
    let output = stdout_actor::new(bufsz); // print state changes
    let director_w_sqlite = director_w_sqlite::new(namespace.namespace, bufsz, Some(output));
    let json_decoder_actor = json_decoder::new(bufsz, director_w_sqlite); // parse input
    let input = stdin_actor::new(bufsz, json_decoder_actor); // read from stdin
    let read_cmd = Message::ReadAllCmd {};
    match input.ask(read_cmd).await {
        IsCompleteMsg {} => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

fn inspect(path: NvPath, bufsz: usize, runtime: Runtime) {
    let result = run_async_inspect(path, bufsz);
    runtime.block_on(result).expect("An error occurred");
}

async fn run_async_inspect(path: NvPath, bufsz: usize) -> Result<(), String> {
    let output = stdout_actor::new(bufsz); // print state
    let director = director_w_sqlite::new(path.path.clone(), bufsz, None);
    let inspect_cmd = Message::InspectCmd { path: path.path };
    match director.ask(inspect_cmd).await {
        m => {
            output.tell(m).await;
            // send complete to keep the job running long enough to print the above
            let complete_cmd = Message::IsCompleteMsg {};
            output.ask(complete_cmd).await;
            Ok(())
        }
    }
}

fn main() {
    env_logger::init();
    debug!("nv started");

    let cli = Cli::parse();
    let bufsz: usize = cli.buffer.unwrap_or(8);

    let runtime = Runtime::new().unwrap_or_else(|e| panic!("Error creating runtime: {}", e));

    match cli.command {
        Commands::Update(namespace) => update(namespace, bufsz, runtime),
        Commands::Inspect(path) => inspect(path, bufsz, runtime),
    }
    debug!("nv stopped");
}
