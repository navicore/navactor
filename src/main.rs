use clap::{CommandFactory, Parser};
use navactor::cli::ifc::{Cli, Commands};
use navactor::cli::runner::{
    configure, explain, inspect, print_completions, run_serve, update, OptionVariant,
};
use navactor::io::net::api_server::HttpServerConfig;
use tokio::runtime::Runtime;
use tracing::info;

fn match_command(pcli: Cli, runtime: &Runtime, memory_only: Option<OptionVariant>, bufsz: usize) {
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
            let server_config = HttpServerConfig::new(port, interface, external_host, namespace);
            run_serve(
                server_config,
                runtime,
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
                runtime,
                silent,
                memory_only,
                wal,
                disable_duplicate_detection,
            );
        }
        Commands::Inspect { path } => inspect(path, bufsz, runtime),
        Commands::Explain { path } => explain(path, bufsz, runtime),
        Commands::Configure { path, gene } => configure(path, gene, bufsz, runtime),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
        }
    }

    info!("nv stopped.");
}

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

    match_command(pcli, &runtime, memory_only, bufsz);
    info!("nv stopped.");
}
