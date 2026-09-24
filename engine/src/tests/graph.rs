use super::*;

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
fn diamond_dependency_macro() -> Result {
    declare_tags!(A, B, C, D);

    let g = orc!(
        a: A; d: D;

        [a] -> B -> [d];
        [a] -> C -> [d];
    );

    let sorted: Vec<_> = g
        .sort_ordered()?
        .into_iter()
        .map(|id| g.meta()[id].type_name())
        .collect();

    // A must be first, D must be last
    assert_eq!(sorted[0], A);
    assert_eq!(sorted[3], D);

    // B and C must be in the middle slots (1 and 2)
    assert!(sorted[1] == B || sorted[1] == C);
    assert!(sorted[2] == B || sorted[2] == C);

    Ok(())
}

/// - G: `(a | b) -> @H -> c`
/// - H: `x -> y`
/// - Combined: `(a | b) -> x -> y -> c`
#[test]
fn merge_at_fork_macro() -> Result {
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
        .sort_ordered()?
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, tags![A, B, X, Y, C]);

    Ok(())
}

/// - G: `Enter -> ( A | @H ) -> Exit`
/// - H: `X -> Y`
/// - Combined: `Enter -> (A | X -> Y) -> Exit`
#[test]
fn embed_graph_inside_fork_group_macro() -> Result {
    declare_tags!(X, Y);
    declare_tags!(Enter, A, Exit);

    let h = orc!(
        X -> Y;
    );

    let g = orc!(
        Enter -> ( A | @h ) -> Exit;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()?
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order[0], Enter);
    assert!(order[1] == A || order[1] == X);
    assert!(order[2] == A || order[2] == X || order[2] == Y);
    assert_eq!(*order.last().unwrap(), Exit);

    Ok(())
}

#[test]
fn standalone_embedding_macro() -> Result {
    declare_tags!(A, B);
    let h = orc!(A -> B;);

    let g = orc!(
        @h;
    );

    let order: Vec<&'static str> = g
        .sort_ordered()?
        .iter()
        .map(|&id| g.meta()[id].type_name())
        .collect();

    assert_eq!(order, tags![A, B]);

    Ok(())
}

#[test]
fn composite_bounds_inference() -> Result {
    declare_tags!(A, B, C, D, X, Y);
    fn _assert_join_join<I: Entry, O: Exit>(_: &Graph<I, O>) {}
    fn _assert_fork_fork<I: GroupEntry, O: GroupExit>(_: &Graph<I, O>) {}
    fn _assert_join_fork<I: Entry, O: GroupExit>(_: &Graph<I, O>) {}
    fn _assert_fork_join<I: GroupEntry, O: Exit>(_: &Graph<I, O>) {}

    let g = orc!(
        a: A -> B;
        [a] -> C;
    );
    _assert_join_fork(&g);

    let g = orc!(
        A -> b: B;
        C -> [b];
    );
    _assert_fork_join(&g);

    let g = orc!(
        x: X; y: Y;
        [y] -> [x];
        [y] -> [x];
    );
    _assert_join_join(&g);

    let g = orc!(
        X;
    );
    _assert_join_join(&g);

    let g = orc!(
        _a: A; _b: B;
        _x: X; _y: Y;
    );
    _assert_fork_fork(&g);

    let g = orc!(
        X; Y;
    );
    _assert_fork_fork(&g);

    let g = orc!(
        X;
        Y;
    );
    _assert_fork_fork(&g);

    let g = orc!(
        A -> B;
        X -> Y;
    );
    _assert_fork_fork(&g);

    let g = orc!(
        a: A -> B -> d: D;
        [a] -> C -> [d];
    );
    _assert_join_join(&g);

    let g = orc!(
        a: A; b: B;
        c: C; d: D;

        [a] -> [b] -> [d];
        [a] -> [c] -> [d];
    );
    _assert_join_join(&g);

    Ok(())
}
