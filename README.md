Navactor
============

Under construction - see [NOTES.md](NOTES.md)

# NOT READY FOR REAL WORK

Available as a crate: https://crates.io/crates/navactor

Overview
----------

A CLI *nix-style tool as lab for actor programming.

NOT TRYING TO BE A FRAMEWORK - the use of actors in `navactor` is in support of
an opinionated experimental approach to modeling and inference processing, not a
general purpose solution for concurrency, parallelism, or distributed computing.

`nv`'s purpose: ingest piped streams of CRLF-delimited observations, send them
to actors, implement the
[OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api)
processing, and persist.

The `nv` command will eventually also work as a networked API server but the
initial model for workflow and performance is data-wrangling via the classic
powerful and undefeated
[awk](https://www.gnu.org/software/gawk/manual/gawk.html).

The ideas that inspire Navactor and DtLab come from CS insights from the early
eighties around [tuple spaces](https://en.wikipedia.org/wiki/Tuple_space) for
coordination languages and later the
[actor](https://en.wikipedia.org/wiki/Actor_model) programming model.

Status
----------

Just a toy implementation in the beginning stages to validate implementation
choices (Rust, Tokio, Sqlite, and Petgraph).

Current functionality is limited to the support of "gauge" and "counter" observations
presented in the internal observation json format via *nix piped stream.

```json
{ "path": "/actors/two", "datetime": "2023-01-11T23:17:57+0000", "values": {"1": 1, "2": 2, "3": 3}}
{ "path": "/actors/two", "datetime": "2023-01-11T23:17:58+0000", "values": {"1": 100}}
{ "path": "/metadata/mainfile", "datetime": "2023-01-11T23:17:59+0000", "values": {"2": 2.1, "3": 3}}
{ "path": "/actors/two", "datetime": "2023-01-11T23:17:59+0000", "values": {"2": 2.98765, "3": 3}}
```

Event sourcing via an embedded sqlite store works.  Query state and resuming
ingestion across multiple runs works.

Using the [observation generator](tests/data/gen_1000.py) in the
[tests/data](tests/data/gen_1000.py) dir, the current impl when run in sqlite
"write ahead logging" mode (WAL), processes and persists 2000+ observations a
second in a tiny disk and memory and cpu footprint.

Messy but working code - I am learning Rust as I recreate the ideas from the
[DtLab Project](https://home.dtlaboratory.com).  However,
[Clippy](https://github.com/navicore/navactor/security/code-scanning) is happy
with the code.

My intention is to support all the features of [DtLab
Project](https://home.dtlaboratory.com) - ie: networked REST-like API and
outward webhooks for useful stateful IOT-ish applications.

Install
----------

```bash
#latest stable version via https://crates.io/crates/navactor
cargo install navactor

#or from this repo:
cargo install --path .
```

enable zsh tab completion:
```bash
nv completions -s zsh > /usr/local/share/zsh/site-functions/_nv
```

Usage
----------

if running from source, replace `nv` with `cargo run --`

```bash
#help
nv -h

#create an actor with telemetry
cat ./tests/data/single_observation_1_1.json | nv update actors

# inspect the state of the actor
nv inspect /actors/one

cat ./tests/data/single_observation_1_2.json | nv update actors
cat ./tests/data/single_observation_1_3.json | nv update actors
nv inspect /actors/one
cat ./tests/data/single_observation_2_2.json | nv update actors
cat ./tests/data/single_observation_2_3.json | nv update actors

```

The above creates a db file named after the namespace - root of any actor path.
In this case, the namespace is 'actors'.

Enable logging via:
```bash
#on the cli
cat ./tests/data/single_observation_1_3.json | RUST_LOG="debug,sqlx=warn" nv update actors

#or set and forget via
export RUST_LOG="debug,sqlx=warn"
```

Developing
-----------

Run all tests via:
```bash
cargo test
```

Run specific tests with logging enabled:
```bash
# EXAMPLE - runs the json decoder and assertions around datetime and json unmarshalling
# the --nocapture lets the in app logging log according to the RUST_LOG env var (see above)
cargo test --test test_json_decoder_actor -- --nocapture
```

----------

`nv` was bootstrapped from Alice Ryhl's very instructive blog post
https://ryhl.io/blog/actors-with-tokio
