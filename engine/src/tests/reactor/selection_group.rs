use super::*;

#[test]
fn base() -> Result<(), ReactorError> {
    declare_tags!(A, B, C, X, Y);

    #[graph(X -> (A ? B ? C) -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = map_nodes(reactor.node_meta().as_slice());

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started | NodeStatus::Resolved(Resolution::Reset) => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, C, X, Y);

    reactor.start()?;
    reactor.resolve(cmd::advanced(map[X], map[B]))?;
    reactor.resolve(cmd::backtracked())?;
    reactor.resolve(cmd::advanced(map[X], map[C]))?;
    reactor.resolve(cmd::resolved(map[C]))?;

    logger.check_log(vec![
        (map[X], NodeStatus::Started),
        (map[B], NodeStatus::Started),
        (map[X], NodeStatus::Resolved(Resolution::Reset)),
        (map[X], NodeStatus::Started),
        (map[C], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
    ]);
    Ok(())
}

#[test]
fn sequential_selection() -> Result<(), ReactorError> {
    declare_tags!(A, B, C, D, X, Y);

    #[graph(X -> (A ? B) -> (C ? D) -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = map_nodes(reactor.node_meta().as_slice());

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, C, D, X, Y);

    reactor.start()?;
    reactor.resolve(cmd::advanced(map[X], map[B]))?;
    reactor.resolve(cmd::advanced(map[B], map[D]))?;
    reactor.resolve(cmd::resolved(map[D]))?;

    logger.check_log(vec![
        (map[X], NodeStatus::Started),
        (map[B], NodeStatus::Started),
        (map[D], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
    ]);

    Ok(())
}
