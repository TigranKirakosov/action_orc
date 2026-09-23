# Engine
Consumes a compiled graph to produce either of two runtime schedule handles:
- single-thread only reactor
- thread-sync orchestrator
Either are intended to be polled in a loop.

## Execution model
- the schedule advances in [waves](action_orc/docs/glossary.md#waves)
- a wave begins when the scheduler determines a set of nodes whose dependencies are all satisfied
- every node in the wave starts concurrently
- the wave ends when every started node in it has been resolved; the next wave is then computed and started
- node order within a wave is not significant; which wave a node belongs to is determined entirely by the graph topology

## Scheduler
An entity holding a graph, validating API commands and driving graph state machine and generating node events.
Graph is held as an immutable blueprint, used to check meta-data or to reconstruct state upon backtracking/advancing, or to reconstruct the whole schedule upon restart.

### Schedule execution flow
Graph: `(A | B) -> C -> (X | Y)`
1) `schedule_start` -> `A` and `B` are started concurrently and both received control, `C` and the rest of downstream are waiting
2) `resolve(A)` -> `A` is resolved, `B` is processing; the rest unchanged
3) `resolve(B)` -> `B` is resolved, next wave is computed (`C`) -> `C` is started; the rest unchanged
4) `resolve(C)` -> `C` is resolved, next wave is computed (`X` and `Y`) -> `X` and `Y` are started concurrently
5) entire schedule is resolved when both `X` and `Y` are resolved, in any order
6) \* -- if the schedule was configured to loop, it would restart; otherwise, terminated

> **Important**: user **must** not forget to resolve each node upon receiving control, otherwise the engine will stall.

## API

### Event Subscription
reactor only: subscribe on `TypeId`'s directly to recieve events
orchestrator: accumulates ordered events in an internal queue; user must drain these an map to own resolvers

### Commands
The engine accepts a set of commands

#### Resolve
`X -> (A | B) -> Y`\
When either of the nodes in the above graph are holding control, user issues `resolve` to proceed to the next group.

- Case X: `X` is holding control -> `resolve(X)` -> `X` is resolved, `A` and `B` are started in parallel, `Y` is dormant.
- Case (A | B): `A` and `B` are holding control concurrently:
    - `resolve(A)` -> `A` is resolved, `B` is processing, `Y` is still dormant;
    - `resolve(B)` -> `B` is resolved, `Y` is started
- Case Y: `Y` is holding control -> `resolve(Y)` -> `Y` is resolved, schedule is over

**Rules**:
- a selector node must not issue `resolve`, it is restricted to issue `advance` only (so that user couldn't skip selection options)

#### Advance
`X -> (A ? B ? C) -> (D ? E) -> Y`\
Similar to `resolve`, but requires a [selector](action_orc/docs/glossary.md#selector) node to hold a control. In the above graph, every node to the left of selection group (e.g., `A ? B`) is considered a selector node.
The idea is that selector, upong resolving, picks up a node to advance schedule to from a list in a selection group. The rest of the group is discarded from the schedule.

- Case X: `X` is holding control -> `advance(B)` -> `X` is resolved, `B` is started, `A` and `C` are discarded; the rest is waiting
- Case B: `B` is now holding control -> `advance(E)` -> `B` is resolved, `E` is started, `D` is discarded and `Y` is dormant

**Rules**:
- selector can't be a member of fork execution group
    - i.e., forbidden topology: `(X | Y) -> (A ? B)`)
- selection group members can't consist of any graph with entry being a fork group
    - i.e., forbidden topology: `X -> ((A | B) ? C)`
    - but this is allowed `X -> (A -> (B | C) ? D)` since subgraph `A -> (B | C)` has single entry `A`

#### Backtrack
`X -> (A ? B) -> C -> Y`\
A node holding control can issue a backtrack command to resolve with reset, while starting upstream node.

- Case B (given it was previously selected by X): `B` is holding control -> `backtrack` -> `B` is resolved with reset, `A` is restored, `X` is started; the rest are waiting
- Case C (given it was previously selected by B): `C` is holding control -> `backtrack` -> `C` is resolved with reset, `B` is started; the rest are waiting

**Rules**:
- single node graph can't issue `backtrack`
- to issue `backtrack`, pivot node (control holder) must have predecessor node, and it must not be a member of a fork group

## Design rationale

### Why Resolve is separate from Advance?
`resolve` command is intended for normal schedule flow resolution - it's a blind operation just to release control over current node.\
`advance` command is context-bound resolver: it resolves current node aswell, but also requires user to provide precise node for branching the schedule.\

### Backtrack?
`backtrack` command is designed for the use cases like `Settings -> (Sound ? Graphics ? Gameplay) -> ...`:\
when entering `Graphics` and resolving it, it's more intutive to immediately return to `Settings` in order to try other options instead of proceeding past the selection group and driving whole schedule to the end to start over from `Settings` again.

### Selector must Advance?
Yes. `selector` is only allowed to be resolved via `advance` command because it must "select" a node from a downstream (`X -> (A ? B)`); if it was allowed to issue `resolve`, it'd be undefined behavior as for what next wave is:
- is it fork/sequential execution of `A` and `B`?
- does only first/last node in a group start?
Thus, `selector` node could only ever be resolved via `advance` command.
