use core::*;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use action_orc_core::*;
use action_orc_macros::*;
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
    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = log.clone();
    let logger = Arc::new(move |id, status| {
        log_clone.lock().unwrap().push((id, status));
    });

    reactor
        .listen_for(TypeId::of::<A>(), logger.clone())
        .unwrap();
    reactor
        .listen_for(TypeId::of::<B>(), logger.clone())
        .unwrap();
    reactor
        .listen_for(TypeId::of::<C>(), logger.clone())
        .unwrap();
    assert_eq!(*log.lock().unwrap(), vec![]);

    reactor.start().unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        vec![(map.fetch("A"), NodeStatus::Started)]
    );

    reactor
        .resolve(map.fetch("A"), Resolution::Finished)
        .unwrap();
    assert_eq! {
        *log.lock().unwrap(),
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
        *log.lock().unwrap(),
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
        *log.lock().unwrap(),
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
    mock_listeners!(
        reactor,
        no_op_listener,
        Enter,
        SpawnEnemies,
        Fight,
        RollLoot,
        PickTreasure
    );
    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = log.clone();
    reactor
        .listen_for(TypeId::of::<Exit>(), move |id, status| {
            log_clone.lock().unwrap().push((id, status));
        })
        .unwrap();

    reactor.start().unwrap();
    for id in &["Enter", "SpawnEnemies", "Fight", "RollLoot"] {
        reactor
            .resolve(map.fetch(id), Resolution::Finished)
            .unwrap();
    }

    assert!(log.lock().unwrap().is_empty(), "Exit blocked");
    reactor
            .resolve(map.fetch("PickTreasure"), Resolution::Finished)
            .unwrap() // last task before Exit
    ;
    assert_eq!(
        *log.lock().unwrap(),
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
            Enter -> @a -> @b -> Exit;
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
    mock_listeners!(
        reactor,
        no_op_listener,
        Enter,
        SpawnEnemies,
        Fight,
        RollLoot,
        PickTreasure
    );
    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let log_clone = log.clone();
    reactor
        .listen_for(TypeId::of::<Exit>(), move |id, status| {
            log_clone.lock().unwrap().push((id, status));
        })
        .unwrap();

    reactor.start().unwrap();
    for id in &["Enter", "SpawnEnemies", "Fight", "RollLoot"] {
        reactor
            .resolve(map.fetch(id), Resolution::Finished)
            .unwrap();
    }

    assert!(log.lock().unwrap().is_empty(), "Exit blocked");

    reactor
        .resolve(map.fetch("PickTreasure"), Resolution::Finished)
        .unwrap(); // last task before Exit
    assert_eq!(
        *log.lock().unwrap(),
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

/// - G: `(a | b) -> @H -> c`
/// - H: `x -> y`
/// - Combined: `(a | b) -> x -> y -> c`
#[test]
fn merge_into_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let h = orc!(
        X -> Y;
    );

    // (a | b) -> @H -> c
    let g = orc!(
        (A | B) -> @h -> C;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B", "X", "Y", "C"])
}

/// - G: `Enter -> ( A | @H ) -> Exit`
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
        Enter -> ( A | @h ) -> Exit;
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
/// - G: `@H -> (A | B) -> C`
/// - Combined: `X -> Y -> (A | B) -> C`
#[test]
fn embed_graph_fan_out_to_parallel_set_macro() {
    declare_tags!(X, Y);
    declare_tags!(A, B, C);

    let h = orc!(
        X -> Y;
    );

    let g = orc!(
        @h -> (A | B) -> C;
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
        @h;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()
        .unwrap()
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, vec!["A", "B"]);
}

/// Verifies [AsGraphProxy] and '@' expression prefix work in conjuction
#[test]
fn struct_expression() {
    declare_tags!(R, A, B, X, Y, X1, Y1);

    struct Race<'a> {
        a: &'a Graph,
        b: &'a Graph,
    }

    impl<'a> AsGraphEntryProxy<'a> for Race<'a> {
        fn as_entry_proxy(self) -> GraphEntry<'a> {
            // Should work too
            // let g = orc! {
            //     R -> ( @self.a | @self.b )
            // };

            let Self { a, b } = self;

            let g = orc! {
                R -> ( @a | @b )
            };

            GraphEntry::OwnedGraph(g)
        }
    }

    fn composer(x: &Graph, y: &Graph) -> Graph {
        orc!(
            A -> @Race { a: x, b: y } -> B;
        )
    }

    let x = orc!(X -> X1;);
    let y = orc!(Y -> Y1;);

    let graph = composer(&x, &y);

    let mut reactor = Reactor::from(graph);
    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let commands = Arc::new(Mutex::new(Vec::new()));

    let log_clone = log.clone();
    let commands_clone = commands.clone();

    let listener = move |id, status| match status {
        NodeStatus::Started => {
            log_clone.lock().unwrap().push((id, status));
            commands_clone
                .lock()
                .unwrap()
                .push((id, Resolution::Finished));
        }
        _ => {}
    };
    mock_listeners!(reactor, listener, R, A, B, X, Y, X1, Y1);

    reactor.start().unwrap();
    drain_commands(&mut reactor, commands);

    let log = log.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("R"), NodeStatus::Started),
            (map.fetch("X"), NodeStatus::Started),
            (map.fetch("Y"), NodeStatus::Started),
            (map.fetch("X1"), NodeStatus::Started),
            (map.fetch("Y1"), NodeStatus::Started),
            (map.fetch("B"), NodeStatus::Started),
        ]
    );
}

/// An attribute-macro #[graph(...)] twin to [struct_expression]
#[test]
fn struct_expression_attribute_macro() {
    declare_tags!(R, A, B, X, Y, X1, Y1, O, K);

    #[graph(R -> (@a | @b))]
    #[params(a, b)]
    struct Race;

    #[graph(O -> K)]
    struct JustAContainer;

    #[graph(A -> @Race { a: x, b: y } -> JustAContainer -> B)]
    #[params(x, y)]
    struct Composer;

    let x = &orc!(X -> X1;);
    let y = &orc!(Y -> Y1;);

    let graph = Composer { x, y };

    let mut reactor = Reactor::from(graph);

    // JustAContainer shouldn't be registered as node, hence not expected
    let expected_types = ["R", "A", "B", "X", "Y", "X1", "Y1", "O", "K"];
    let missing_types: Vec<&'static str> = reactor
        .node_meta()
        .iter()
        .map(|(_, meta)| meta.type_name())
        .filter(|t| !expected_types.contains(&t))
        .collect();

    assert!(
        missing_types.is_empty(),
        "Graph compiled without expected meta types: {missing_types:#?}"
    );

    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let commands = Arc::new(Mutex::new(Vec::new()));

    let log_clone = log.clone();
    let commands_clone = commands.clone();

    let listener = move |id, status| match status {
        NodeStatus::Started => {
            log_clone.lock().unwrap().push((id, status));
            commands_clone
                .lock()
                .unwrap()
                .push((id, Resolution::Finished));
        }
        _ => {}
    };
    mock_listeners!(reactor, listener, R, A, B, X, Y, X1, Y1, O, K);

    reactor.start().unwrap();
    drain_commands(&mut reactor, commands);

    let log = log.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("R"), NodeStatus::Started),
            (map.fetch("X"), NodeStatus::Started),
            (map.fetch("Y"), NodeStatus::Started),
            (map.fetch("X1"), NodeStatus::Started),
            (map.fetch("Y1"), NodeStatus::Started),
            (map.fetch("O"), NodeStatus::Started),
            (map.fetch("K"), NodeStatus::Started),
            (map.fetch("B"), NodeStatus::Started),
        ]
    );
}

/// Verifies #[graph(...)] works on unit structs
#[test]
fn unit_struct_expression_derive_macro() {
    declare_tags!(A, B, X, Y);

    #[graph(A -> (X | Y) -> B)]
    struct Unit;

    let mut reactor = Reactor::from(Unit);
    let map = map_nodes(reactor.node_meta().as_slice());

    let log = Arc::new(Mutex::new(Vec::new()));
    let commands = Arc::new(Mutex::new(Vec::new()));

    let log_clone = log.clone();
    let commands_clone = commands.clone();

    let listener = move |id, status| match status {
        NodeStatus::Started => {
            log_clone.lock().unwrap().push((id, status));
            commands_clone
                .lock()
                .unwrap()
                .push((id, Resolution::Finished));
        }
        _ => {}
    };
    mock_listeners!(reactor, listener, A, B, X, Y);

    reactor.start().unwrap();
    drain_commands(&mut reactor, commands);

    let log = log.lock().unwrap().clone();
    assert_eq!(
        log,
        vec![
            (map.fetch("A"), NodeStatus::Started),
            (map.fetch("X"), NodeStatus::Started),
            (map.fetch("Y"), NodeStatus::Started),
            (map.fetch("B"), NodeStatus::Started),
        ]
    );
}

#[test]
fn orchestrator_schedule_loop() {
    declare_tags!(A, B, C);

    #[graph(A -> B -> C)]
    struct Graph;

    let mut orchestrator = Orchestrator::new(Graph, ScheduleConfig { should_loop: true }).unwrap();

    let resolver = orchestrator.resolver();
    let map = map_nodes(orchestrator.node_meta().as_slice());

    orchestrator.start().unwrap();

    let is_complete = orchestrator.tick().unwrap();
    assert!(
        !is_complete,
        "Schedule should not be marked done while threads are processing"
    );

    let wave_1_events = orchestrator.drain_events();
    assert_eq!(wave_1_events, vec![(map.fetch("A"), NodeStatus::Started),]);

    resolver
        .send((map.fetch("A"), Resolution::Finished))
        .unwrap();

    let is_complete = orchestrator.tick().unwrap();
    assert!(!is_complete);

    let wave_2_events = orchestrator.drain_events();
    assert_eq!(
        wave_2_events,
        vec![
            (map.fetch("A"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("B"), NodeStatus::Started),
        ]
    );

    resolver
        .send((map.fetch("B"), Resolution::Finished))
        .unwrap();

    orchestrator.tick().unwrap();
    let wave_3_events = orchestrator.drain_events();
    assert_eq!(
        wave_3_events,
        vec![
            (map.fetch("B"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("C"), NodeStatus::Started),
        ]
    );

    resolver
        .send((map.fetch("C"), Resolution::Finished))
        .unwrap();

    orchestrator.tick().unwrap();
    let wave_4_events = orchestrator.drain_events();
    assert_eq!(
        wave_4_events,
        vec![(map.fetch("C"), NodeStatus::Resolved(Resolution::Finished)),]
    );

    let is_complete = orchestrator.tick().unwrap();
    assert!(
        !is_complete,
        "Loop schedule resets instead of exiting execution"
    );

    let loop_reset_events = orchestrator.drain_events();
    assert_eq!(
        loop_reset_events,
        vec![(map.fetch("A"), NodeStatus::Started),]
    );
}

// F is blocked by [e]
// [e] is blocked by both [b] and [d]
// [d] is blocked by C
// Topological order:
// A, (B, C, D or C, D, B or C, B, D), E, F
#[test]
fn orchestrator_topological_correctness() {
    declare_tags!(A, B, C, D, E, F);

    #[graph(
        a: A -> (b: B | C -> d: D);
        ([b] | [d]) -> e: E;
        [e] -> F;
    )]
    struct Graph;

    let mut orchestrator = Orchestrator::new(Graph, ScheduleConfig { should_loop: true }).unwrap();

    let resolver = orchestrator.resolver();
    let map = map_nodes(orchestrator.node_meta().as_slice());

    orchestrator.start().unwrap();

    let is_complete = orchestrator.tick().unwrap();
    assert!(!is_complete);

    let events = orchestrator.drain_events();
    assert_eq!(events, vec![(map.fetch("A"), NodeStatus::Started)]);

    resolver
        .send((map.fetch("A"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map.fetch("A"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("B"), NodeStatus::Started),
            (map.fetch("C"), NodeStatus::Started),
        ]
    );

    // TEST NON-DETERMINISM: resolve the C -> D before B finishes
    resolver
        .send((map.fetch("C"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map.fetch("C"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("D"), NodeStatus::Started),
        ]
    );

    resolver
        .send((map.fetch("D"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![(map.fetch("D"), NodeStatus::Resolved(Resolution::Finished)),]
    );

    resolver
        .send((map.fetch("B"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map.fetch("B"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("E"), NodeStatus::Started),
        ]
    );

    resolver
        .send((map.fetch("E"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map.fetch("E"), NodeStatus::Resolved(Resolution::Finished)),
            (map.fetch("F"), NodeStatus::Started),
        ]
    );

    resolver
        .send((map.fetch("F"), Resolution::Finished))
        .unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![(map.fetch("F"), NodeStatus::Resolved(Resolution::Finished)),]
    );

    let is_complete = orchestrator.tick().unwrap();
    assert!(
        !is_complete,
        "Loop schedule resets instead of exiting execution"
    );

    let events = orchestrator.drain_events();
    assert_eq!(events, vec![(map.fetch("A"), NodeStatus::Started),]);
}

trait Mapping<K, V> {
    fn fetch(&self, key: K) -> V;
}

impl Mapping<&'static str, NodeId> for HashMap<&'static str, NodeId> {
    fn fetch(&self, key: &'static str) -> NodeId {
        let sentinel = usize::MAX;
        self.get(key).copied().unwrap_or(sentinel)
    }
}

fn drain_commands(reactor: &mut Reactor, commands: Arc<Mutex<Vec<(NodeId, Resolution)>>>) {
    loop {
        let pending: Vec<(NodeId, Resolution)> = {
            let mut guard = commands.lock().unwrap();
            std::mem::take(&mut *guard)
        };

        if pending.is_empty() {
            break;
        }

        for (id, resolution) in pending {
            let _ = reactor.resolve(id, resolution);
        }
    }
}

#[test]
fn orchestrator_multiple_roots() {
    declare_tags!(RootX, RootY, BarrierNode);

    #[graph(
        RootX -> b: BarrierNode;
        RootY -> [b];
    )]
    struct ParallelRootsGraph;

    let mut orchestrator =
        Orchestrator::new(ParallelRootsGraph, ScheduleConfig { should_loop: false }).unwrap();
    let map = map_nodes(orchestrator.node_meta().as_slice());

    orchestrator.start().unwrap();
    orchestrator.tick().unwrap();

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map.fetch("RootX"), NodeStatus::Started),
            (map.fetch("RootY"), NodeStatus::Started),
        ]
    );
}

#[test]
fn orchestrator_no_loop_termination() {
    declare_tags!(X, Y);

    #[graph(X -> Y)]
    struct Graph;

    let mut orchestrator = Orchestrator::new(Graph, ScheduleConfig { should_loop: false }).unwrap();
    let resolver = orchestrator.resolver();
    let map = map_nodes(orchestrator.node_meta().as_slice());

    orchestrator.start().unwrap();

    orchestrator.tick().unwrap();
    orchestrator.drain_events();
    resolver
        .send((map.fetch("X"), Resolution::Finished))
        .unwrap();

    orchestrator.tick().unwrap();
    orchestrator.drain_events();
    resolver
        .send((map.fetch("Y"), Resolution::Finished))
        .unwrap();

    orchestrator.tick().unwrap();
    let subsequent_events = orchestrator.drain_events();
    assert_eq!(
        subsequent_events,
        vec![(map.fetch("Y"), NodeStatus::Resolved(Resolution::Finished))]
    );

    let is_complete = orchestrator.tick().unwrap();
    assert!(
        is_complete,
        "Orchestrator must return true signaling the schedule is over"
    );
}

fn map_nodes(node_meta: &[(NodeId, &Meta)]) -> HashMap<&'static str, NodeId> {
    let mut s2i = HashMap::new();
    for (id, meta) in node_meta {
        s2i.insert(meta.type_name(), *id);
    }

    s2i
}

fn no_op_listener(_: usize, _: NodeStatus) {}

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
        ($reactor:expr, $listener:expr, $($tag:ident),* $(,)?) => {
            $(
                $reactor.listen_for(std::any::TypeId::of::<$tag>(), $listener.clone()).unwrap();
            )*
        };
    }
    pub(crate) use mock_listeners;
}
