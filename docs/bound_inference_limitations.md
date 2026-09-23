# Bound Inference Limitations

The `orc!` macro infers the `<I, O>` type-state of the resulting `Graph<I, O>`
using a **syntactic** heuristic — it checks whether the first or last AST node of
the graph is a fork group `(A | B)` or whether multiple top-level graph lines
are present (`a: A; b: B; [a] -> C; [b] -> C;`).

The automatic inference works for inline fork groups and multi-line declarations.\
When an **embedded graph** sits at the entry or exit boundary of the outer graph, compiler will mark respective edge as `Ambiguous`.

## Ambiguity cases

```rust
let forky: Graph<Fork, Single> = orc!((X | Y) -> Z);

let sinky: Graph<Single, Single> = orc!(A -> B);

// Entry edge - `@forky` is opaque to graph, compiler stamps `Ambiguous` at entry bound:
let graph: Graph<Ambiguous, Single> = orc!(@forky -> C);

// Exit edge - `@sinky` is opaque to graph aswell, `Ambiguous` is stamped at the exit bound:
let graph: Graph<Single, Ambiguous> = orc!(A -> @sinky -> @forky);

// Both edges:
let graph: Graph<Ambiguous, Ambiguous> = orc!(@forky -> A -> @sinky);
```

## Disambiguation via annotations

```rust
let graph: Graph<Fork, Single> = orc!(@[Fork; Single] forky -> C);
//                                          ^^^^^^^^^^^^^^ - hint compiler with forky's bound types

let graph: Graph<Single, Single> = orc!(A -> @sinky -> @[Fork; Single] forky);
//                                          ^^^^^^^ - annotation is unnecessary here

// Both edges:
let graph: Graph<Fork, Single> = orc!(@[Fork; Single] forky -> A -> @[Single; Single] sinky);

```

## Inference mechanics

Compiler takes into account following graph properties during AST walking to infer final graph bounds:
- is entry a fork: set to `true` only when the *first* node is a `NodeExpr::Group { mode: Fork }`
- is exit a fork: set similarly for the *last* node
- total lines per graph: for multi-line graphs

An `@ident` expression or a bare `#[graph]` unit-struct ident is parsed as a
`NodeExpr::Expression` or `NodeExpr::Declaration`.\

They're opaque to compiler, so without user annotations their bounds default to `Ambiguous`.

The compiler **cannot inspect the type of `@emb`** at proc-macro expansion time, it only sees tokens.\
There is no mechanism to ask "does this `Graph<I,O>` have `I = Fork`?" at macro time,
thus any embedding flavor is opaque to the compiler,
and without user annotations their bounds default to `Ambiguous`.

## Runtime implications

**Runtime is unaffected.** The `I`/`O` type parameters on `Graph<I, O>`
are phantom, they're never checked against the actual graph structure.
`GraphBuilder::build::<Single, Single>()` succeeds even with 10 sources and 20 sinks.
The runtime engine (`Schedule`, `Reactor`, `Orchestrator`) works correctly regardless of what `I`/`O` is stamped on the `Graph`.

It only matters when consumer code dispatches on the `Graph<I, O>` type:

```rust
fn run_parallel(engine: &Graph<Fork, Single>) { /* ... */ }

let g = orc!(@forky -> C); // inferred as Graph<Ambiguous, Single>
run_parallel(&g);          // compile error: Ambiguous != Fork
```
