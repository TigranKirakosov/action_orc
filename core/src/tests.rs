use crate::{utils::get_type_name, *};

#[test]
fn simple_graph() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = PlainGraph::new();
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
        tags![A]
    );
    assert_eq!(
        g.sinks()
            .map(|id| g.meta()[id].type_name())
            .collect::<Vec<_>>(),
        tags![F]
    );
}

#[test]
fn topological_sort() {
    declare_tags!(A, B, C, D, E, F);

    let mut g = PlainGraph::new();
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
        tags![A, B, C, E, D, F]
    );
}

#[test]
fn disjoint_sets() {
    declare_tags!(A, B, X, Y);

    let mut g = PlainGraph::new();
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
        sorted.iter().position(|&name| name == A).unwrap()
            < sorted.iter().position(|&name| name == B).unwrap()
    );
    assert!(
        sorted.iter().position(|&name| name == X).unwrap()
            < sorted.iter().position(|&name| name == Y).unwrap()
    );
}

#[test]
fn cyclic_graph_returns_err() {
    declare_tags!(A, B, C);

    let mut g = PlainGraph::new();

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

    let empty_g = PlainGraph::new();
    assert_eq!(empty_g.sort_ordered().unwrap(), vec![]);

    let mut single_g = PlainGraph::new();
    add_nodes!(single_g, a: A);

    let sorted = single_g.sort_ordered().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(single_g.meta()[sorted[0]].type_name(), A);
}

#[test]
fn diamond_dependency() {
    declare_tags!(A, B, C, D);

    let mut g = PlainGraph::new();
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
    assert_eq!(sorted[0], A);
    assert_eq!(sorted[3], D);

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == B || sorted[1] == C);
    assert!(sorted[2] == B || sorted[2] == C);
}

/// G: (a | b) -> c
/// H: x -> y
#[test]
fn merge_at_fork_group() {
    declare_tags!(A, B, C);
    declare_tags!(X, Y);

    // (a | b) -> c
    let mut g = Graph::<Fork, Join>::new();
    add_nodes! {
          g,
          a: A, b: B, c: C,
    };
    g.add_edge(a, c);
    g.add_edge(b, c);

    // x -> y
    let mut h = PlainGraph::new();
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

    assert_eq!(order, tags![A, B, X, Y, C])
}
