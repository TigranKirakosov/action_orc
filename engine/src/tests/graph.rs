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
fn merge_into_parallel_set_macro() -> Result {
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
fn embed_graph_inside_parallel_group_macro() -> Result {
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
fn parallel_group_compile_marker() -> Result {
    declare_tags!(X, Y);
    declare_tags!(A0, A1, B0, B1);

    // This graph has two parallel tracks
    // i.e., two sources and two sinks
    // let precompiled = orc!(
    //     A0 -> A1;
    //     B0 -> B1;
    // );

    let precompiled = orc!(
        X -> (A0 -> A1 ? B0 -> B1) -> Y;
    );

    // Can't allow this due to parallel groups are not a subject for selection
    let attempt_to_select_predefined = orc!(
      X -> (@precompiled ? Y);
    );

    Ok(())
}
