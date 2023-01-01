mod actor;
mod extractor_actor;
mod messages;
mod stdin_actor;
mod stdout_actor;
use crate::extractor_actor::ExtractorActorHandle;
use crate::messages::ActorMessage::IsCompleteMsg;
use crate::stdin_actor::StdinActorHandle;
use crate::stdout_actor::StdoutActorHandle;
use clap::{Args, Parser, Subcommand};
use log::debug;
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
    /// Define extractor from incoming stream of rules
    Define(Extractor),
    /// List extractors
    List(NoArg),
    /// Process incoming stream of internal formats
    Update(NoArg),
    /// Process incoming stream of external formats with extractor
    Ingest(Extractor),
    /// Get actor state for all actors in path
    Inspect(Inspect),
}

#[derive(Args)]
struct NoArg {}

#[derive(Args)]
struct Inspect {
    /// actor path
    path: String,
}

#[derive(Args)]
struct Extractor {
    /// name of extractor
    extractor: String,
}

fn define(spec: Extractor, bufsz: usize, runtime: Runtime) {
    let result = run_async_define(spec, bufsz);
    runtime.block_on(result).expect("An error occurred");
}

async fn run_async_define(spec_holder: Extractor, bufsz: usize) -> Result<(), String> {
    let extractor = ExtractorActorHandle::new(bufsz);

    match extractor.define(spec_holder.extractor).await {
        IsCompleteMsg { respond_to_opt: _ } => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

fn list(_: usize, _: Runtime) {}

fn update(_: usize, _: Runtime) {}

fn inspect(_: Inspect, _: usize, _: Runtime) {}

fn ingest(_: Extractor, bufsz: usize, runtime: Runtime) {
    let result = run_async_ingest(bufsz);
    runtime.block_on(result).expect("An error occurred");
}

async fn run_async_ingest(bufsz: usize) -> Result<(), String> {
    let output = StdoutActorHandle::new(bufsz);
    let input = StdinActorHandle::new(bufsz, output);

    match input.read().await {
        IsCompleteMsg { respond_to_opt: _ } => Ok(()),
        _ => Err("END and response: sucks.".to_string()),
    }
}

fn main() {
    env_logger::init();
    debug!("nv started");
    let cli = Cli::parse();
    let bufsz: usize = cli.buffer.unwrap_or(8);
    let runtime = Runtime::new().unwrap_or_else(|e| panic!("Error creating runtime: {}", e));

    match cli.command {
        Commands::Define(extractor) => define(extractor, bufsz, runtime),
        Commands::List(_) => list(bufsz, runtime),
        Commands::Update(_) => update(bufsz, runtime),
        Commands::Ingest(extractor) => ingest(extractor, bufsz, runtime),
        Commands::Inspect(path) => inspect(path, bufsz, runtime),
    }
    debug!("nv stopped");
}
