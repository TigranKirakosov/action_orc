# Graph Bounds
A graph always has upstream and downstream nodes.\
*Synonims*: entries; sources and sinks; roots and leaves.\
Consider the following graph:
```text
A -> (B | C) -> D
```
- `A` has no upstream nodes and a fork group downstream `(B | C)`; `D` is a downstream node too, just not an immediate one
- `B` and `C` have `A` as an upstream node and `D` as a downstream
- `D` has an upstream fork group `(B | C)` and a distant `A` too

# Graph Entry
A set of traits for converting graph-like structures into graph entries.\
That is, a simple `struct Foo` can be converted to a basic graph node where its source is `Foo` and its sink is `Foo` aswell.
Other examples of a graph-like structures are various instances of a `Graph` itself, be it an owned or borrowed data.
Notable case of a graph is a struct denoted with `#[graph]` attribute macro - it is considered a graph entry due to macro implementing necessary conversion traits on it.

# (Execution) Wave
A causual wave of events in a directed graph. Within a wave, everything is concurrent.\
When a node is started, it receives control and since then must be resolved. Once that node is resolved, downstream nodes in a graph will get started.\
The following graph has 3 waves of execution:
```text
X -> (A | B) -> Y
```
1) `X` is started and then resolved - wave 1 is complete
2) `A` and `B` belong to wave 2, they get started; when they're both resolved, wave 2 is complete
3) `Y` is started and then resolved - wave 3 is complete

# Selector
A node that terminates a selection group.\
A selector's downstream consists of one or more alternate paths it must proceed to; when the selector resolves, it selects which downstream branch to advance into via `Advance` command.\
If a node's downstream is a selection group `(A ? B ? C)`, that node is a selector.

# Selection Group
A group of `2 >=` graphs separated by `?`:
```text
(A ? B ? C) -> X
```
This group is considered resolved when a selected node is resolved; the rest nodes are discarded and wont be processed.\
That is, `X` will start once any of `A`, `B` or `C` is resolved.

# Fork Group
A group of `2 >=` graphs separated by `|`:
```text
(A | B | C) -> X
```
Each node in this group runs concurrently, hence resolved independently.\
This group is considered resolved when all listed nodes are resolved.\
That is, `X` will start once all of `A`, `B` and `C` are resolved.

# Sequence Group
A group of `2 >=` graphs separated by `,`:
```text
(A, B, C) -> X
```
Each node in this group runs and resolves sequentially.\
That is, `X` will start once all of `A`, `B` and `C` are resolved.\

*Note*: we could express same graph from the above with this syntax:
```text
A -> B -> C -> X
```
