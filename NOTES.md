Navactor Technical Approach
====================

A CLI tool as lab for the actor use cases.  Ingest piped streams of CRLF-
delimited observations, send them to actors, implement the [OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api) 
processing, and persist.

Re-implement the DtLab DtPath with a graph lib (petgraph) and fully support graphs instead of trees.

Postpone network support and make a dtlab cli lab that is a useful personal
data science tool.

----------

Parts
----------

Parts: `rust, clap, tokio, rusqlite, tokio-rusqlite, petgraph`

Maybe `rayon` but only for operator apply when a single actor's self update
will benefit from parallelism.

Concepts and Values
---------------

1. cli-first
2. messages can be any serializable object (unparsed msg is ok)
3. actors can ONLY persist NUMERICAL DATA
3. persistence can only be of event source commands (state changes) - consider shallow clones around time windows rather than the akka snapshot approach
4. all work done by actors must be done with DtLab Operators
5. all IO must be via actor messaging
  a. even stdin be read from an input actor
  b. state changes optionally published to an output / publishing actor and that first impl does stdout

USAGE
---------------

all state be a single db file

```bash
cat my.jsonl | nv -d ./my.db > my-new-state-log.jsonl
```

TODO
--------

1. ~~clap~~
2. ~~ingest stdin stream into actor msgs~~
3. ~~parse msgs in actor impls~~
4. enable https://github.com/tokio-rs/console
5. next internal msg format needs source timestamp, nv timestamp, path.
  a. nv timestamp in envelope
  b. nv source timestamp and path in payload hierarchy

MORE
---------

* Operator expression could be processed an embedded lang runtime or a new DSL

