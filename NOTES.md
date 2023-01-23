Navactor Technical Approach
====================

A CLI tool as lab for the actor use cases.  Ingest piped streams of CRLF-
delimited observations, send them to actors, implement the [OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api) 
processing, and persist.

Re-implement the DtLab DtPath with a graph lib (petgraph) and fully support graphs instead of trees.

Postpone network support and make a dtlab cli lab that is a useful personal
data science tool.

Eventually, with the addition of networking and sharding and replication, I hope
to make IoT *seem like* an embarrassingly parallel problem where there is no
special care given to coordinating inputs or queries, even though under
the covers there is serial workflow.

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

DESIGN IN PROGRESS
--------

### types and operators

from chatGPT:

```
Does "odometer vs speedometer" suggest "accumulator vs guage"?

Yes, "odometer" and "speedometer" can be used as an analogy for "accumulator"
and "gauge" respectively. An odometer is a device that measures the distance
traveled by a vehicle and accumulates the distance traveled over time and a
speedometer measure the instantaneous speed of a vehicle. In the same way, an
accumulator in your telemetry system measures the accumulated count over time,
and a gauge in your system measures the instantaneous value.
```

### biology metaphors and patters for inspiration

protocells have 3 things to be "alive", ie: the functionalities for life:

1. genes ( control via mutable inheritable information )
2. container (identity, localization, reproduction )
3. metabolism ( resource transformation, self-mainenance growth )

Genees in this system is the type and its mappings for operators.  The type
metadata is data that can be edited ( mutated, randomly mutated ) as it is
used as a prototype for offspring.

Containers for us are the actor instance.  Ideally the actor would manage its
own persistance - but for now it gets help with director and store_actor - need
bio metaphors for director and store_actor.

Metabolism is message passing - internal automatic messages, external observation
delivery, self initiated queries of or expressions of interest in other actors.

TODO: what's missing to go to an agent metaphor?  How can an actor move itself
from one namespace and host to another?

TODO
--------

1. ~~clap~~
2. ~~ingest stdin stream into actor msgs~~
3. ~~parse msgs in actor impls~~
4. ~~next internal msg format needs source timestamp, nv timestamp, path.~~
  a. ~~nv timestamp in envelope~~
  b. ~~nv source timestamp and path in payload hierarchy~~
  c. ~~unmarshal time strings~~
5. ~~sqlx and sqlite~~
6. REFACTOR - learn to make smaller async methods
7. cli control of db file and actor namespaces
8. default type that indicates which numeric fields are accumulators and which are gauges - this implies the built-in operator
9. code other built-ins for ranges of keys for the default type

MORE
---------

* Operator expression could be processed an embedded lang runtime or a new DSL
* enable https://github.com/tokio-rs/console

