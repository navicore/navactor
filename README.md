Navactor
============

Under construction - see [NOTES.md](NOTES.md)

# NOT READY FOR REAL WORK

Available as a crate: https://crates.io/crates/navactor

Overview
----------

A CLI *nix-style tool as lab for actor programming.

Mission: Ingest piped streams of CRLF-delimited observations, send them to actors,
implement the [OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api) 
processing, and persist.

The `nv` command will eventually also work as a networked API server but the
initial model for workflow and performance is data-wrangling via the classic
powerful and undefeated [awk](https://www.gnu.org/software/gawk/manual/gawk.html).

The ideas that inspire Navactor and DtLab come from CS insights from the early
eighties around [tuple spaces](https://en.wikipedia.org/wiki/Tuple_space) for coordination languages and then later around 
the [actor](https://en.wikipedia.org/wiki/Actor_model) programming model - and now also influenced by emerging
industry [IOT medadata](https://infoscience.epfl.ch/record/273579?ln=en) standards encoded in RDF that suggest dynamic graphs of
[digital twins](https://en.wikipedia.org/wiki/Digital_twin).

![Fun Mutation of DtLab Graphic](images/dtlab-mutant-3.png)

Status
----------

Just a toy implementation in the beginning stages to validate implementation
choices (Rust and Sqlite and Petgraph).

Current functionality is limited to the support of "gauge" observations
presented in the internal observation json format via *nix piped stream.

Event sourcing via an embedded sqlite store works.  Query state and resuming
ingestion across multiple runs works.

Messy but working code - I am learning Rust as I recreate the ideas from
the [DtLab Project](https://home.dtlaboratory.com).  However, [Clippy](https://github.com/navicore/navactor/security/code-scanning) is happy with the code.

My intention is to support all the features of
[DtLab Project](https://home.dtlaboratory.com) - ie: networked REST-like API and
outward webhooks for useful stateful IOT-ish applications.

Install
----------

```bash
#latest stable version via https://crates.io/crates/navactor
cargo install navactor

#or from this repo:
cargo install --path .
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

![Fun Mutation of DtLab Graphic](images/diodes-2.png)
----------

`nv` was bootstrapped from Alice Ryhl's very excellent and instructive blog post https://ryhl.io/blog/actors-with-tokio
