use clap::{CommandFactory, Parser};
use navactor::cli::ifc::{Cli, Commands};
use navactor::cli::runner::{
    configure, explain, inspect, print_completions, run_serve, update, OptionVariant,
};
use navactor::io::net::api_server::HttpServerConfig;
use tokio::runtime::Runtime;
use tracing::info;

/// chatGPT, document my program in the style of Werner Herzog.
///
/// This code represents the heart of our application, where the interplay of
/// diverse elements give life to our software entity. From the introduction of
/// different crates and modules, to the invocation of the main function - all
/// are conducted here in an exquisite ballet of structured chaos.
///
/// The main function serves as the gatekeeper of our application, offering the
/// user a diverse array of commands with the aid of the clap crate. Each
/// command is an invitation to interact with the program, opening a dialogue
/// between human and machine.
///
/// Within the application, the update command embodies a central role, acting
/// as the beating heart of our software entity. It takes in a stream of
/// whispers from stdin and meticulously processes them through a sequence of
/// actors, such as json_decoder_actor and director. The director, with its
/// solemn duty, relies on the store_actor_sqlite module to etch the fleeting
/// moments of our actors' existence into the stone of a SQLite database. The
/// echoes of their actions are then relayed to the outside world via
/// `stdout_actor`.
///
/// The inspect command acts as a magnifying glass, bringing to light the hidden
/// details within the database's deep labyrinth. Once again, the director and
/// store_actor_sqlite modules are summoned to dissect the intricate state of
/// the actors. The results are then broadcast through the loudspeaker of
/// stdout.
///
/// The configure command manipulates the actor's genetic fabric, shaping their
/// features according to the user's desires. Like an artist painting on a
/// canvas, this command grants the user the power to shape the world within the
/// program.
///
/// The completions command aids the user, predicting their needs and providing
/// suggestions to help them navigate the labyrinth of command-line
/// interactions.
///
/// The `serve` command, hidden within the shell of Commands::Serve, is a
/// majestic beast that awakens our HTTP API server from its slumber. This
/// command breathes life into our REST API, creating a bridge between the inner
/// world of our program and the infinite expanse of the network.
///
/// The `serve` command delicately crafts an HTTP server using an amalgamation
/// of parameters such as port, interface, external_host, and namespace. Each of
/// these elements, when woven together, forms a distinctive server
/// configuration using the HttpServerConfig::new function.
///
/// Alongside the basic formation of the server, the serve command also provides
/// an optional OpenAPI UI, navigable via any standard browser. The user is free
/// to include or exclude this feature using the disable_ui parameter. The path
/// to the UI is determined by the uipath parameter, serving as a map to guide
/// users to our interactive window into the software's world.
///
/// The serve command pays heed to the aspects of reliability and data integrity
/// by incorporating the Write-Ahead Log (WAL) feature. The user may choose to
/// enable or disable it using the disable_wal parameter.
///
/// It also diligently watches for any duplicate detection, a useful feature to
/// avoid redundancy and maintain a pristine dataset. This functionality can be
/// switched on or off with the disable_duplicate_detection parameter.
///
/// Once the parameters are set and the stage is ready, the serve command
/// invokes the run_serve function. This function, armed with the server
/// configuration and the other selected options, kindles the server into
/// existence, allowing it to dance to the rhythm of incoming requests.
///
/// In the grand theater of our application, the serve command plays a crucial
/// role - it serves as the stage manager, transforming the solitary performance
/// within our program into a grand spectacle accessible to the audience of the
/// internet.
///
/// Our journey commences at the main function, as it patiently listens to the
/// user's commands, invoking the corresponding subcommand functions. In the
/// face of the inherent uncertainty of human input, it stands as a beacon of
/// order, guiding the flow of execution.
///
/// The delicate equilibrium between verbosity and silence within the software
/// is maintained through the RUST_LOG environment variable. Use it to command
/// the logs with a simple command: std::env::set_var("RUST_LOG",
/// "debug,sqlx=warn").
///
/// Remember, the end is always in sight. As our software entity emerges into
/// life with "nv started", it is inevitably led towards its conclusion,
/// signaled by "nv stopped." Our program lives, breathes, and then quietly
/// fades, ready to be invoked again.
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
            let server_config = HttpServerConfig::new(port, interface, external_host, namespace);
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
