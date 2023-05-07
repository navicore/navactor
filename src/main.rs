use clap::{CommandFactory, Parser};
use navactor::cli::*;
use navactor::cli_interface::*;
use tokio::runtime::Runtime;

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
/// The `serve` command is launches a REST API and OpenAPI UI is optionally available via browser.
///
/// The entry point for the application is the main function which reads the command-line arguments
/// and invokes the appropriate `subcommand` functions.
///
/// control logging of nv and various libs via `RUST_LOG` env var like so:
/// `std::env::set_var("RUST_LOG`", "debug,sqlx=warn");
fn main() {
    env_logger::init();
    log::info!("nv started");

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
            uipath,
            disable_ui,
        } => run_serve(&runtime, port, interface, external_host, uipath, disable_ui),
        Commands::Update {
            namespace,
            silent,
            wal,
            allow_duplicates,
        } => {
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
        Commands::Explain { path } => explain(path, bufsz, &runtime),
        Commands::Configure { path, gene } => configure(path, gene, bufsz, &runtime),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
        }
    }

    log::info!("nv stopped.");
}
