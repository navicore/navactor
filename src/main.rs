mod actor;
mod lineiter;
mod messages;
mod stdinactor;
mod stdoutactor;
use crate::actor::ActorHandle;
use crate::stdinactor::StdinActorHandle;
use crate::stdoutactor::StdoutActorHandle;
use clap::{Args, Parser, Subcommand};
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
    /// Process incoming stream of with extractor
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

fn main() {
    let cli = Cli::parse();
    let bufsz: usize = cli.buffer.unwrap_or(8);
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let output = StdoutActorHandle::new(bufsz);
        let input = StdinActorHandle::new(bufsz, output);
        let r = input.read().await;
        println!("response: {}", r);
    });

    let a = ActorHandle::new();
    let _ = a.get_unique_id();

    //todo: instantiate input actor
    //todo: instantiate output actor
    //todo: send start command to input actor

    //todo: need some lifecycle - perhaps block on shutdown for output actor and output actor only
    //shuts down because input actor sent and EOF msg to output actor?
}
