# Macros

## `orc!`
Inline macro; produces a compiled graph

## `#[graph]`
Struct attribute macro.
Implements conversion traits on a given struct so the compiler sees it as a first-class graph-type, enabling `@Embedding` feature at use sites.

A unit struct decorated with `#[graph]` becomes an embeddable sub-graph and may be referenced inside `orc!` *without* the `@` prefix - it gets inlined into its parent:

```rust
#[graph(O -> K)]
struct JustAContainer;

#[graph(A -> JustAContainer -> B)]
struct Composer; // expands A -> (O -> K) -> B
```

Structs with fields must declare them manually (with their own lifetime and type guards) and are only usable via the `@` embedding operator:

```rust
#[graph(R -> (@a | @b))]
struct Race<'a> {
    a: &'a Graph<Join, Join>,
    b: &'a Graph<Join, Join>,
}
```

## Parser
Reads custom DSL and builds a recursive AST.\
`winnow` for structural parsing, `syn` to preserve type spans to keep inference intact with LSPs.

## AST
Is recursive to support nested subraph.\
Each line inside proc-macro DSL is considered a graph.\
Each node in a syntax slot, i.e., in place of any letter...
```rust
let emb = orc!(D);
let graph = orc!(
    b: B;
    A -> ([b] | C -> (@emb ? E));
);
```
...is considered a graph, be it:
- declaration: `A`
- binding declaration: `b: B`
- bound-reference: `[b]`
- composite: `C -> (...)`
- or embedded: `@emb`

## Compiler
- maintains DAG invariants by running Kahn's topological sort on every edge
- emits error messages tied to precise token spans
- ouputs a graph with [bounds meta-data](action_orc/core/README.md#Builder).
