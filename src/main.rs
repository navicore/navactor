mod actor;
use crate::actor::ActorHandle;
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

    if cli.verbose >= 1 {
        println!("buffer: {:?}", cli.buffer);
        println!("verbose: {:?}", cli.verbose);
    }
    println!("dbfile: {:?}", cli.dbfile);

    // let runtime = Runtime::new().unwrap();
    // let r = runtime.block_on(async {
    //     let a = ActorHandle::new();
    //     let r = a.get_unique_id().await;
    //     r
    // });
    // println!("response: {}", r);

    //todo: instantiate input actor
    //todo: instantiate output actor
    //todo: send start command to input actor

    //todo: need some lifecycle - perhaps block on shutdown for output actor and output actor only
    //shuts down because input actor sent and EOF msg to output actor?
}
