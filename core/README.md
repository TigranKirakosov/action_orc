# Core

## Graph
A DAG (Directed Acyclic Graph) data structure and a set of algorithms to build a graph and maintain DAG properties.
Each node in a graph is directly tied to Rust types (via `TypeId`) to later allow subscription on node lifecycle events.

## Builder
A compile-time tool to build and compose graph while stamping meta-data later to be consumed in `macros` crate at codegen.\
It consumes [graph entries](action_orc/docs/glossary.md#GraphEntry) (that is, anything convertable to [graph-bounds](action_orc/docs/glossary.md#GraphBounds)), accumulating graph-edges and meta-data, and emits a final graph on build call.\
Role meta-data of graph bounds is used to allow enforcement of graph composition rules according to DSL.
