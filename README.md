Navactor
============

Under construction - see [NOTES.md](NOTES.md)

# NOT READY FOR REAL WORK YET

Overview
----------

A CLI tool as lab for the actor programming use cases.

Ingest piped streams of CRLF- delimited observations, send them to actors,
implement the [OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api) 
processing, and persist.

The `nv` command will eventually also serve a networked API but the initial 
model for workflow and performance is data-wrangling via the classic powerful `awk`.

Status
----------

The current functionality is limited to support of "guage" observations
presented in the internal observation json format via cli piped stream.

Event sourcing from embedded sqlite store works.  Query state and resuming
ingestion across multiple runs works.

Messy but working code - I am learning Rust as I recreate the ideas from
the [DtLab Project](https://home.dtlaboratory.com).  However, [Clippy](https://github.com/navicore/navactor/security/code-scanning) is happy with the code.

The plan is to support all the features of [DtLab Project](https://home.dtlaboratory.com) - ie: networked REST-like API and outward webhooks for useful stateful IOT-ish applications.

Install
----------

```bash
cargo install --path .

or

cargo install navactor
```


Usage
----------

```bash
#help
nv -h

#create an actor with telemetry
cat ./tests/data/single_observation_1_1.json | cargo run -- update actors

# inspect the state of the actor
cargo run -- inspect /actors/one

cat ./tests/data/single_observation_1_2.json | cargo run -- update actors
cat ./tests/data/single_observation_1_3.json | cargo run -- update actors
cargo run -- inspect /actors/one
cat ./tests/data/single_observation_2_2.json | cargo run -- update actors
cat ./tests/data/single_observation_2_3.json | cargo run -- update actors

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

----------

`nv` was bootstrapped from Alice Ryhl's very excellent and instructive blog post https://ryhl.io/blog/actors-with-tokio
