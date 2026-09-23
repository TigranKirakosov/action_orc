# An Action Orchestrator (WIP)

A declarative DSL for composing hierarchical execution graphs in Rust.

`action-orc` provides a straightforward framework:
- Document action flows in a static, yet composable graph notation
- Define event resolvers to map custom app logic to designated actions
- Drive action transitions with customizable gating logic

## Planned Features
- [ ] fluent and complete:
    - [ ] IDE support
    - [ ] compilation error spans
    - [ ] compile-time detection of circular dependencies
- [ ] graph visualizer

## Planned Integrations
- [ ] [Bevy plugin](TODO: put link to plugin crate)

## Syntax Overview
> Declare your battle formations with the `orc!` macro, and let the Warchief lead the horde to victory!

- Declare nodes: bind handles (`node: Marker`) or match anonymously (`Marker`)
- Map dependencies: `lhs -> rhs` (i.e., **lhs** blocks **rhs**)
- Bind already declared nodes: `[b] -> y`
- Compose graphs: dynamically embed sub-graphs (`A -> sub -> B`)
- Group nodes into ordered *Sequences* or *Fork* branches:
    - Sequence block: `(a, b, c)`
    - Fork block: `(x | y | z)`

```rust
use action_orc::*;

fn warchief_campaign(reinforce: &Graph) -> Graph {
    orc! {
        // Define first timeline
        BuildCamp -> (
            gather: GatherResources,
            // make reinforce dependant on upstream nodes
            (Defend | RequestReinforcements) -> @reinforce,
        );

        // Define second fork timline, linked with the first one by [gather] and @reinforce nodes
        prepare: PrepareCampaign -> (
            [gather] -> BuildWarmachines
            | TrainGrunts
            | @reinforce // make this timline dependant on @reinforce aswell
        );

        // Declare exit node
        victory: CelebrateVictory;

        // Define path to exit
        [prepare] -> AssembleArmy -> LaunchCampaign -> [victory];
    }
}
```

## License

Copyright © 2026 Tigran Kirakosov.

This project is dual-licensed under either:

* **MIT License** ([LICENSE-MIT](LICENSE-MIT))
* **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
