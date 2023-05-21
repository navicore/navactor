use clap::{CommandFactory, Parser};
use navactor::api_server::HttpServerConfig;
use navactor::cli::{
    configure, explain, inspect, print_completions, run_serve, update, OptionVariant,
};
use navactor::cli_interface::{Cli, Commands};
use tokio::runtime::Runtime;
use tracing::info;

/// This is the `main` entry point for the application. It imports various Rust crates and a set of
/// local modules that constitute the program.
///
/// The `main` function of this module is to provide the command-line interface for the application.
/// It uses the clap crate to generate commands and `subcommands`.
///
/// The `update` command is the core functionality of the application, which updates the state of the
/// actors by processing input messages from `stdin`. The `stdin_actor` module reads input messages
/// from `stdin`, which are then passed through a chain of processing actors, such as
/// `json_decoder_actor` and ``director``. The director uses a `store_actor_sqlite` module to persist the
/// updated state of the actors in a `SQLite` database. The updated state is then sent to the
/// `stdout_actor` module, which writes output to 'stdout'.
///
/// The `inspect` command inspects the state of a particular path in the database. It uses the
/// `director` and `store_actor_sqlite` modules to query the state of the actors and writes the output
/// to `stdout`.
///
/// The `configure` command allows for configuring actor properties as genes.
///
/// The `completions` command is used by shell completion functionality to generate command-line
/// completion suggestions.
///
/// The `serve` command is launches a REST API and `OpenAPI` UI is optionally available via
/// browser.
///
/// The entry point for the application is the main function which reads the command-line arguments
/// and invokes the appropriate `subcommand` functions.
///
/// control logging of nv and various libs via `RUST_LOG` env var like so:
/// `std::env::set_var("RUST_LOG`", "debug,sqlx=warn");
fn main() {
    tracing_subscriber::fmt::init();
    info!("This will be logged to stdout");
    info!("nv started");

    let pcli = Cli::parse();
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
        Commands::Serve {
            port,
            interface,
            external_host,
            namespace,
            uipath,
            disable_ui,
            disable_wal,
            disable_duplicate_detection,
        } => {
            let wal = match disable_wal {
                Some(true) => OptionVariant::Off,
                _ => OptionVariant::On,
            };
            let disable_duplicate_detection = match disable_duplicate_detection {
                Some(false) => OptionVariant::Off,
                _ => OptionVariant::On,
            };
            let server_config = HttpServerConfig {
                port,
                interface,
                external_host,
                namespace,
            };
            run_serve(
                server_config,
                &runtime,
                uipath,
                disable_ui,
                wal,
                disable_duplicate_detection,
            );
        }
        Commands::Update {
            namespace,
            silent,
            disable_wal,
            disable_duplicate_detection,
        } => {
            let silent = match silent {
                Some(true) => OptionVariant::On,
                _ => OptionVariant::Off,
            };
            let memory_only = memory_only.unwrap_or(OptionVariant::Off);
            let wal = match disable_wal {
                Some(true) => OptionVariant::Off,
                _ => OptionVariant::On,
            };
            let disable_duplicate_detection = match disable_duplicate_detection {
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
                disable_duplicate_detection,
            );
        }
        Commands::Inspect { path } => inspect(path, bufsz, &runtime),
        Commands::Explain { path } => explain(path, bufsz, &runtime),
        Commands::Configure { path, gene } => configure(path, gene, bufsz, &runtime),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
        }
    }

    info!("nv stopped.");
}
