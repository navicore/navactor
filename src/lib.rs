//! `chatGPT`, document my program in the style of Werner Herzog.
//!
//! Welcome to `NavActor`, our application, a living testament to the marriage of chaos and order.
//! Through the intricate interplay of its modules and actors, `NavActor` transcends the binary
//! world, constructing a universe within the bounds of our program.
//!
//! In `NavActor`, the diverse array of commands offer themselves as conduits for user interaction,
//! facilitating a dialogue between human and machine, mediated by the intuitive interface of
//! the `cli` module.
//!
//! Our application's lifeblood is the `update` command, the beating heart of `NavActor`.
//! Whispered inputs from stdin are transformed into a chorus by a meticulously coordinated
//! ensemble of actors. They etch the fleeting moments of our actor's existence into the
//! immutable stone of a `SQLite` database with the assistance of the `store_actor_sqlite`
//! module, their actions resonating through the `stdout_actor`.
//!
//! The `inspect` command unravels the hidden details within the labyrinth of the database.
//! Yet again, our diligent actors are summoned to dissect the intricate state of the database
//! before broadcasting their findings through stdout, their words a testament to the truths
//! concealed within the depths of the data.
//!
//! The `configure` command shapes the world within our program, allowing users to sculpt the
//! actor's features in accordance with their desires. Like an artist with a blank canvas, this
//! command grants users the power to manipulate the genetic fabric of the actors.
//!
//! The `completions` command aids the user, predicting their needs and providing suggestions
//! to guide them through the maze of command-line interactions. This assistant, subtle yet
//! indispensable, eases the journey within `NavActor`.
//!
//! Concealed within the `Commands::Serve`, the `serve` command awakens our HTTP API server from
//! its digital slumber. The server, once stirred, forms a bridge between the serene inner world
//! of our program and the infinite expanse of the network.
//!
//! Alongside the basic formation of the server, the serve command also provides an optional
//! `OpenAPI` UI, navigable via any standard browser. This user interface, a map guiding users
//! to the treasure trove of software's capabilities, can be included or excluded at the user's whim.
//!
//! The serve command diligently guards against redundancy, keeping a watchful eye on any
//! duplicates to maintain a pristine dataset. Its vigilance ensures data integrity and reliability
//! by incorporating the Write-Ahead Log (WAL) feature, which can be enabled or disabled at will.
//!
//! Upon setting the stage, the serve command kindles the server into existence with the `run_serve`
//! function. Armed with the server configuration and other selected options, the server dances
//! to the rhythm of incoming requests, echoing the harmony of user interactions.
//!
//! `NavActor`'s modules work in concert, performing a symphony of functions under the masterful
//! baton of the `main` function, maintaining order amidst the inherent uncertainty of user input.
//!
//! The balance between verbosity and silence within `NavActor` is maintained through the
//! `RUST_LOG` environment variable. It wields the power to command the logs with a simple
//! incantation: `std::env::set_var("RUST_LOG", "debug,sqlx=warn")`.
//!
//! As `NavActor` breathes to life with "nv started", it dances a ballet of functions before inevitably
//! being led towards its conclusion, signaled by "nv stopped." Our program is a phoenix, living,
//! breathing, and then quietly fading, only to be ready to rise again from its own ashes.
pub mod actors;
pub mod cli;
pub mod io;
pub mod utils;
