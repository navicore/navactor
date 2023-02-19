//! Navactor makes use of the `Clap` library to define and parse command-line arguments. The `CLI`
//! is used to manage and interact with actors in a distributed system.
//!
//! The main struct defined in this code is `Cli`, which is derived from the `Parser` and `Debug`
//! traits provided by Clap. The `Cli` struct defines the command-line arguments that the program
//! expects to receive, such as the name of the event store file, the size of the actor mailbox,
//! and the verbosity level of the program's logging.
//!
//! The `Cli` struct also defines a command field that holds a variant of the `Commands` enum,
//! which is also derived from the `Subcommand` and Debug traits provided by Clap. The `Commands`
//! enum represents the different `subcommands` that the program can accept, such as Update,
//! Inspect, `Configure`, and `Completions`.
//!
//! Each variant of the `Commands` enum defines its own set of command-line arguments that are
//! specific to that `subcommand`. For example, the Update variant has several arguments such as
//! `silent`, `wal`, `namespace`, and `allow_duplicates`, while the `Configure` variant has path
//! and gene arguments.
//!
//! Finally, the `NoArgs` struct is defined, which is used to represent a command that takes no
//! arguments.
//!
//! Overall, this Rust code defines the command-line interface for a distributed system management
//! tool, allowing users to interact with actors in the system by issuing commands and passing
//! arguments. The code makes use of the `Clap` library to define and parse the command-line
//! arguments, making it easy for users to get up and running with the tool quickly and
//! efficiently.

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "nv",
    author,
    version,
    about,
    long_about = "nv is the CLI for the DtLaboratory project",
    propagate_version = true
)] // Read from `Cargo.toml`
pub struct Cli {
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
    pub buffer: Option<usize>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "No on-disk db file", long_help = "For best performance, but you should not run with '--silent' as you won't know what the in-memory data was since it is now ephemeral.")]
    pub memory_only: Option<bool>,
    // #[command(subcommand)]
    // pub command: Option<Commands>,
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Update {
        #[arg(short, long, action = clap::ArgAction::SetTrue, help = "No output to console.", long_help = "Supress logging for slightly improved performance if you are loading a lot of piped data to a physical db file.")]
        silent: Option<bool>,
        #[arg(short, long, action = clap::ArgAction::SetTrue, help = "Write Ahead Logging", long_help = "Enable Write Ahead Logging (WAL) for performance improvements for use cases with frequent writes")]
        wal: Option<bool>,
        #[arg(short, long, action = clap::ArgAction::Set, long_help = "the director and db file to default to")]
        namespace: Option<String>,
        #[arg(short,long, action = clap::ArgAction::SetTrue, help = "Accept path+datetime collisions", long_help = "The journal stores and replays events in the order that they arrive but will ignore events that have a path and observation timestamp previously recorded - this is the best option for consistency and performance.  With 'disable-duplicate-detection' flag, the journal will accept observations regardless of the payload timestamp - this is good for testing and best for devices with unreliable notions of time.")]
        allow_duplicates: Option<bool>,
    },
    Inspect {
        #[arg(short, long, action = clap::ArgAction::Set, help = "get the state of an actor")]
        path: String,
    },
    Configure {
        #[arg(short, long, action = clap::ArgAction::Set, help = "set the gene for the path", long_help = "Set the gene for a path of actors - '/' being root and all actors, /somepath will set the gene for all actors under 'somepath', overriding the root gene.")]
        path: String,
        #[arg(short, long, action = clap::ArgAction::Set)]
        gene: String,
    },
    Completions {
        #[arg(short, long, action = clap::ArgAction::Set, help = "print script for shell tab completion", long_help = "Pipe the output of this command to a file or to a shell program as appropriate for 'bash', or 'zsh', etc... install via 'nv completions -s zsh > /usr/local/share/zsh/site-functions/_nv'")]
        shell: clap_complete::Shell,
    },
}

#[derive(Args, Debug)]
struct NoArgs {}
