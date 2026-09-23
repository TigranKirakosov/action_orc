# Overview
The framework could be split into two parts:
- compile-time: DAG toplogy declaration language based on proc-macro;
- run-time: a stateful orchestrator for multi-threaded environments which provides an event-driven API to drive underlying runtime schedule based on provided compile-time graph

## [Core](action_orc/core/README.md)
This is an internal crate serving as a low-level API for codegen to unroll DSL into Rust at `macros` crate.

### Notable entities
- graph: a plain index-tracking DAG datastructure baked with node meta-data (`TypeId`);
- builder: a helper struct for `macros` crate to generate graph from parsed DSL;
- graph_entry: a collection of type conversion traits to constrain and uniform various types for convenient consumption in `macros` crate at codegen piepline

## [Macros](action_orc/macros/README.md)
Another internal crate where the DSL is implemented in; hosts various types of helper-macros alongside with main proc-macro - `orc!`.
This crate's responsibility is to enforce compile-time graph invariants and DSL rules so that compiled graphs have valid topology so that runtime engine could safely consume and process them.

### Notable entities
- parser: `winnow` + `syn` to preserve LSP intellisense;
- AST: supports recursive structures;
- compiler: unrolls DSL into Rust;
- inline `orc!` proc-macro and struct attribute macro `#[graph]`

## [Engine](action_orc/engine/README.md)
A user API facade and integration test hub for `macros` and `core` crates (it is impossible to test macros inside own proc-macro labeled crate).
All necessary types from other crates must be re-exported to the user from here.

### Notable entities
- schedule: a graph consumer and validator of API commands;
- reactor: a mid-layer of engine and node-observer registry; can be used for single-threaded envs without orchestrator layer (not a priority);
- orchestrator: a thread-sync layer based on event and command queues; no internal background threads - must be polled by calling `Orchestrator::tick()` in a loop within user-defined thread
