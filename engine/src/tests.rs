use core::*;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use action_orc_core::*;
use action_orc_macros::orc;
use local_macros::*;

use crate::*;

#[test]
fn simple_graph() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = Graph::new();
    add_nodes!(
        g,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, d);
    g.add_edge(d, e);
    g.add_edge(e, f);

    assert_eq!(
        g.sources()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        g.sinks()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn simple_graph_macro() {
    declare_tags!(A, B, C, D, E, F);

    let g = orc!(
        A -> B -> C -> D -> E -> F;
    );

    assert_eq!(
        g.sources()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A"]
    );
    assert_eq!(
        g.sinks()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["F"]
    );
}

#[test]
fn topological_sort() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = Graph::new();
    add_nodes!(
        g,
        a: A, b: B, c: C,
        d: D, e: E, f: F
    );

    g.add_edge(a, b);
    g.add_edge(a, c);
    g.add_edge(a, e);
    g.add_edge(b, d);
    g.add_edge(c, d);
    g.add_edge(e, f);

    assert_eq!(
        g.sort_ordered()
            .unwrap()
            .into_iter()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        vec!["A", "B", "C", "E", "D", "F"]
    );
}

#[test]
fn disjoint_sets() {
    declare_tags!(A, B, X, Y);

    let mut g = Graph::new();
    add_nodes!(g,
        a: A, b: B,
        x: X, y: Y
    );

    // Set 1
    g.add_edge(a, b);
    // Set 2
    g.add_edge(x, y);

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // Check upstreams of both sets come before their downstreams
    assert!(
        sorted.iter().position(|&name| name == "A").unwrap()
            < sorted.iter().position(|&name| name == "B").unwrap()
    );
    assert!(
        sorted.iter().position(|&name| name == "X").unwrap()
            < sorted.iter().position(|&name| name == "Y").unwrap()
    );
}

#[test]
fn cyclic_graph_returns_err() {
    declare_tags!(A, B, C);

    let mut g = Graph::new();
    add_nodes!(g, a: A, b: B, c: C);

    // Loop: A -> B -> C -> A
    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, a);

    assert_eq!(g.sort_ordered().err(), Some(GraphError::CycleDetected));
}

#[test]
fn empty_and_single_node() {
    declare_tags!(A);

    let empty_g = Graph::new();
    assert_eq!(empty_g.sort_ordered().unwrap(), vec![]);

    let mut single_g = Graph::new();
    add_nodes!(single_g, a: A);

    let sorted = single_g.sort_ordered().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(single_g.meta()[sorted[0]].type_name(), "A");
}

#[test]
fn diamond_dependency() {
    declare_tags!(A, B, C, D);

    let mut g = Graph::new();
    add_nodes!(g, a: A, b: B, c: C, d: D);

    g.add_edge(a, b);
    g.add_edge(a, c);
    g.add_edge(b, d);
    g.add_edge(c, d);

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == "B" || sorted[1] == "C");
    assert!(sorted[2] == "B" || sorted[2] == "C");
}

#[test]
fn diamond_dependency_macro() {
    declare_tags!(A, B, C, D);

    let g = orc!(
        a: A; d: D;

        [a] -> B -> [d];
        [a] -> C -> [d];
    );

    let sorted: Vec<_> = g
        .sort_ordered()
        .unwrap()
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == "B" || sorted[1] == "C");
    assert!(sorted[2] == "B" || sorted[2] == "C");
}

#[test]
fn lifecycle_hooks() {
    declare_tags!(A, B, C);

    let g = orc!(
        A -> B -> C;
    );

    let mut reactor = Reactor::from(g);
    let map = map_nodes(&reactor);

    let lifecycle_log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = lifecycle_log.clone();
    let lifecycle_logger = Arc::new(move |id, event| {
        log_clone.lock().unwrap().push((id, event));
    });

    reactor
        .listen_for(TypeId::of::<A>(), lifecycle_logger.clone())
        .unwrap();
    reactor
        .listen_for(TypeId::of::<B>(), lifecycle_logger.clone())
        .unwrap();
    reactor
        .listen_for(TypeId::of::<C>(), lifecycle_logger.clone())
        .unwrap();
    assert_eq!(*lifecycle_log.lock().unwrap(), vec![]);

    reactor.init().unwrap();
    assert_eq!(
        *lifecycle_log.lock().unwrap(),
        vec![(map.fetch("A"), NodeStatus::Started)]
    );

    reactor
        .resolve(map.fetch("A"), Resolution::Finished)
        .unwrap();
    assert_eq! {
        *lifecycle_log.lock().unwrap(),
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("A"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("B"), NodeStatus::Started),
        ]
    };

    reactor
        .resolve(map.fetch("B"), Resolution::Finished)
        .unwrap();
    assert_eq! {
        *lifecycle_log.lock().unwrap(),
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("A"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("B"), NodeStatus::Started),
            (map.fetch("B"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("C"), NodeStatus::Started),
        ]
    };

    reactor
        .resolve(map.fetch("C"), Resolution::Finished)
        .unwrap();
    assert_eq! {
        *lifecycle_log.lock().unwrap(),
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("A"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("B"), NodeStatus::Started),
            (map.fetch("B"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("C"), NodeStatus::Started),
            (map.fetch("C"), NodeStatus::Resolved(Resolution::Finished)),
        ]
    };
}

#[test]
fn nested_pipeline_composition() {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    let mut room = Graph::new();
    add_nodes! {
          room,
          enter: Enter, exit: Exit,
    };
    room.add_edge(enter, exit);

    let mut combat = Graph::new();
    add_nodes! {
          combat,
          spawn: SpawnEnemies, fight: Fight,
    };
    combat.add_edge(spawn, fight);

    let mut loot = Graph::new();
    add_nodes! {
          loot,
          roll: RollLoot, pick: PickTreasure,
    };
    loot.add_edge(roll, pick);

    let combat_bounds = room.merge(&combat, vec![enter]);
    let _loot_bounds = room.merge(&loot, combat_bounds.sinks);

    let mut reactor = Reactor::from(room);
    mock_listeners!(reactor, Enter, SpawnEnemies, Fight, RollLoot, PickTreasure);
    let map = map_nodes(&reactor);

    let lifecycle_log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = lifecycle_log.clone();
    reactor
        .listen_for(TypeId::of::<Exit>(), move |id, event| {
            log_clone.lock().unwrap().push((id, event));
        })
        .unwrap();

    reactor.init().unwrap();
    for id in &["Enter", "SpawnEnemies", "Fight", "RollLoot"] {
        reactor
            .resolve(map.fetch(id), Resolution::Finished)
            .unwrap();
    }

    assert!(lifecycle_log.lock().unwrap().is_empty(), "Exit blocked");
    reactor
            .resolve(map.fetch("PickTreasure"), Resolution::Finished)
            .unwrap() // last task before Exit
    ;
    assert_eq!(
        *lifecycle_log.lock().unwrap(),
        vec![(map.fetch("Exit"), NodeStatus::Started)]
    );
}

#[test]
fn nested_pipeline_composition_macro() {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    fn room(a: &Graph, b: &Graph) -> Graph {
        orc! {
            Enter -> a -> b -> Exit;
        }
    }

    let combat = orc!(
        SpawnEnemies -> Fight;
    );

    let loot = orc!(
        RollLoot -> PickTreasure;
    );

    let composed_room = room(&combat, &loot);

    let mut reactor = Reactor::from(composed_room);
    mock_listeners!(reactor, Enter, SpawnEnemies, Fight, RollLoot, PickTreasure);
    let map = map_nodes(&reactor);

    let lifecycle_log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = lifecycle_log.clone();
    reactor
        .listen_for(TypeId::of::<Exit>(), move |id, event| {
            log_clone.lock().unwrap().push((id, event));
        })
        .unwrap();

    reactor.init().unwrap();
    for id in &["Enter", "SpawnEnemies", "Fight", "RollLoot"] {
        reactor
            .resolve(map.fetch(id), Resolution::Finished)
            .unwrap();
    }

    assert!(lifecycle_log.lock().unwrap().is_empty(), "Exit blocked");

    reactor
        .resolve(map.fetch("PickTreasure"), Resolution::Finished)
        .unwrap(); // last task before Exit
    assert_eq!(
        *lifecycle_log.lock().unwrap(),
        vec![(map.fetch("Exit"), NodeStatus::Started)]
    );
}

/// G: (a | b) -> c
/// H: x -> y
#[test]
fn merge_into_parallel_set() {
    declare_tags!(A, B, C);
    declare_tags!(X, Y);

    // (a | b) -> c
    let mut g = Graph::new();
    add_nodes! {
          g,
          a: A, b: B, c: C,
    };
    g.add_edge(a, c);
    g.add_edge(b, c);

    // x -> y
    let mut h = Graph::new();
    add_nodes!(h, x: X, y: Y);
    h.add_edge(x, y);

    // (a | b) -> x -> y -> c
    let _h_bounds = g.merge(&h, vec![a, b]);

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}

/// - G: `(a | b) -> H -> c`
/// - H: `x -> y`
/// - Combined: `(a | b) -> x -> y -> c`
#[test]
fn merge_into_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let h = orc!(
        X -> Y;
    );

    // (a | b) -> H -> c
    let g = orc!(
        (A | B) -> h -> C;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}

/// - G: `Enter -> ( A | H ) -> Exit`
/// - H: `X -> Y`
/// - Combined: `Enter -> (A | X -> Y) -> Exit`
#[test]
fn embed_graph_inside_parallel_group_macro() {
    declare_tags!(X, Y);
    declare_tags!(Enter, A, Exit);

    let h = orc!(
        X -> Y;
    );

    let g = orc!(
        Enter -> ( A | h ) -> Exit;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order[0], "Enter");
    assert!(order[1] == "A" || order[1] == "X");
    assert!(order[2] == "A" || order[2] == "X" || order[2] == "Y");
    assert_eq!(*order.last().unwrap(), "Exit");
}

/// - H: `X -> Y`
/// - G: `H -> (A | B) -> C`
/// - Combined: `X -> Y -> (A | B) -> C`
#[test]
fn embed_graph_fan_out_to_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let h = orc!(
        X -> Y;
    );

    let g = orc!(
        h -> (A | B) -> C;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["X", "Y", "A", "B", "C"]);
}

#[test]
fn standalone_embedding_macro() {
    declare_tags!(A, B);
    let h = orc!(A -> B;);

    let g = orc!(
        h;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B"]);
}

trait Mapping<K, V> {
    fn fetch(&self, key: K) -> V;
}

impl Mapping<&'static str, NodeId> for HashMap<&'static str, NodeId> {
    fn fetch(&self, key: &'static str) -> NodeId {
        self.get(key).copied().unwrap()
    }
}

fn map_nodes(reactor: &Reactor) -> HashMap<&'static str, NodeId> {
    let mut s2i = HashMap::new();
    for (id, meta) in reactor.node_meta() {
        s2i.insert(meta.type_name(), id);
    }

    s2i
}

mod local_macros {
    macro_rules! declare_tags {
        ($($type:ident),* $(,)?) => {
            $(
                struct $type;
            )*
        };
    }
    pub(crate) use declare_tags;

    macro_rules! add_nodes {
        ($graph:expr, $($tag:ident : $type:ty),* $(,)?) => {
            $(
                #[allow(unused)]
                let $tag = $graph.add_node::<$type>();
            )*
        };
    }
    pub(crate) use add_nodes;

    macro_rules! mock_listeners {
        ($reactor:expr, $($tag:ident),* $(,)?) => {
            $(
                $reactor.listen_for(std::any::TypeId::of::<$tag>(), |_, _| {}).unwrap();
            )*
        };
    }
    pub(crate) use mock_listeners;
}
