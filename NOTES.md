Navactor Technical Approach
====================

A CLI tool as lab for the actor programming use cases.  Ingest piped streams of
CRLF- delimited observations, send them to actors, implement the [OPERATOR](https://github.com/DTLaboratory/dtlab-scala-alligator#operator-api) 
processing, and persist.

Re-implement the DtLab DtPath with a graph lib (petgraph) and fully support graphs instead of trees.

~~Postpone network support and make a dtlab cli lab that is a useful personal
data science tool.~~

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
3. persistence can only be of event source commands (state changes) - consider shallow clones around time windows rather than snapshots
4. all work done by actors must be done with DtLab Operators
5. all IO must be via actor messaging
  a. even stdin must be read from an input actor
  b. state changes optionally trigger snapshot publishing
6. actor handler calls are single threaded - a promise from the runtime, though successive handler calls may be on different threads so actor must not do any thread-local assumptions / cheats

USAGE
---------------

all state stored to a single db file

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

Genes in this system is the type and its mappings for operators.  The type
metadata is data that can be edited ( mutated, randomly mutated ) as it is
used as a prototype for offspring.

Containers for us are the actor instance.  Ideally the actor would manage its
own persistence - but for now it gets help with director and store_actor - need
bio metaphors for director and store_actor.  The metadata that supports the
graph of actors and their messaging reinforces that identity and localizing
aspects of the protocell metaphor.

Metabolism is message passing - internal automatic messages, external observation
delivery, self initiated queries of or expressions of interest in other actors.

TODO: what's missing to go to an agent metaphor?  How can an actor move itself
from one namespace and host to another?

### first impl of genes

A gene has:
* a set OR ranges of legal telemetry indexes
* mapping of operators to their input indexes
* mapping of operators to their output indexes
* the GENE-SPEC artifact could be literate programming text+code, something beautiful
* the GENE-SPEC scope is a node on a path.. 
* the instantiated actor gets all the specs its path infers and inheritance is overridden by nearness
* metadata is just data / telemetry arriving - so metatdata has GENE-SPECs too
* metadata can be embedded into the system by any GENE-SPEC and overriden by dynamic metadata

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
7. ~~cli control of db file and actor namespaces~~
8. ~~default type that indicates which numeric fields are accumulators and which are gauges - this implies the built-in operator~~
9. ~~code other built-ins for ranges of keys for the default type~~
10. support different numeric types of actors, f64 vs u32 - tokio blocks us sending T
11. ~~persist path -> gene association~~
12. genes should tell director about time-scoped actors
13. add path + gene mapping to cli args

for #8 above - the default gene is all rust, don't figure out config yet.
* the default gene reserves keys 100 - 199 for guages
* the guages rewrite the index they use as input
* the default gene reserves keys 200 - 199 for accumulators
* the accumulators rewrite the index they use as input

parts for default gene (or any gene):
1. factor interface to get operator with an int (key)
1. operator trait that is a function
  a. input accepts Update msg, Actor state
  b. returns new actor state to be set

performance 
------------

* 1 day 100 devices 10 per sec took 10 minutes to ingest 1.2 million recs.
* 1 day 100 devices 10 per sec took 11 minutes to ingest 1.2 million recs.
* - will try next to run w/o any println output actor
* 1 day 100 devices 10 per sec took 9 minutes to ingest 1.2 million recs.
* - will try next to run w/o any persist actor
* 1 day 100 devices 10 per sec took 1 minutes to ingest 1.2 million recs.
* - will try next to run with WAL journal_mode
* 1 day 100 devices 10 per sec took 7 minutes to ingest 1.2 million recs.
* 36% improvement!


MORE
---------

* Operator expression could be processed an embedded lang runtime or a new DSL
* enable https://github.com/tokio-rs/console

