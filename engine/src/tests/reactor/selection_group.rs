use super::*;

#[test]
fn base() -> Result {
    declare_tags!(A, B, C, X, Y);

    #[graph(X -> (A ? B ? C) -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();

    let (roles_us, roles_ds): (Vec<UpstreamRole>, Vec<DownstreamRole>) = reactor
        .node_meta()
        .into_iter()
        .map(|(_, m)| (m.role_us(), m.role_ds()))
        .unzip();

    assert_eq!(
        roles_us,
        vec![
            UpstreamRole::Selector,
            UpstreamRole::Regular,
            UpstreamRole::Regular,
            UpstreamRole::Regular,
            UpstreamRole::Regular,
        ]
    );

    assert_eq!(
        roles_ds,
        vec![
            DownstreamRole::Regular,
            DownstreamRole::SelectionBranch,
            DownstreamRole::SelectionBranch,
            DownstreamRole::SelectionBranch,
            DownstreamRole::Regular,
        ]
    );

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started | NodeStatus::Resolved(Resolution::Reset) => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, C, X, Y);

    reactor.start()?;
    reactor.process(cmd::advance(map[X], map[B]))?;
    reactor.process(cmd::backtrack())?;
    reactor.process(cmd::advance(map[X], map[C]))?;
    reactor.process(cmd::resolve(map[C]))?;

    logger.check_log(vec![
        (map[X], NodeStatus::Started),
        (map[B], NodeStatus::Started),
        (map[B], NodeStatus::Resolved(Resolution::Reset)),
        (map[X], NodeStatus::Started),
        (map[C], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
    ]);
    Ok(())
}

#[test]
fn sequential_selection() -> Result {
    declare_tags!(A, B, C, D, X, Y);

    #[graph(X -> (A ? B) -> (C ? D) -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, C, D, X, Y);

    reactor.start()?;
    reactor.process(cmd::advance(map[X], map[B]))?;
    reactor.process(cmd::advance(map[B], map[D]))?;
    reactor.process(cmd::resolve(map[D]))?;

    logger.check_log(vec![
        (map[X], NodeStatus::Started),
        (map[B], NodeStatus::Started),
        (map[D], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
    ]);

    Ok(())
}

#[test]
fn selection_to_parallel() -> Result {
    declare_tags!(A, B, X, Y, Z);

    #[graph(X -> (A ? B) -> (Y | Z))]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, X, Y, Z);

    reactor.start()?;
    reactor.process(cmd::advance(map[X], map[A]))?;
    reactor.process(cmd::resolve(map[A]))?;

    logger.check_log(vec![
        (map[X], NodeStatus::Started),
        (map[A], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
        (map[Z], NodeStatus::Started),
    ]);

    Ok(())
}

#[test]
fn advance_bypass() -> Result {
    declare_tags!(A, B, C, X);

    #[graph(X -> (A ? B ? C))]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();
    mock_listeners!(reactor, no_op_listener, A, B, C, X);

    reactor.start()?;

    assert_eq!(
        reactor.process(cmd::resolve(map[X])).err(),
        Some(ReactorError::Scheduler(SchedulerError::AdvanceBypass {
            pivot: NodeDisplay::from(map[X], reactor.get_meta(map[X]))
        }))
    );

    Ok(())
}

#[test]
fn illegal_branch_selection() -> Result {
    declare_tags!(A, B, C, X, Y);

    #[graph(X -> (A ? B ? C) -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();
    mock_listeners!(reactor, no_op_listener, A, B, C, X);

    reactor.start()?;

    assert_eq!(
        reactor.process(cmd::advance(map[X], map[Y])).err(),
        Some(ReactorError::Scheduler(
            SchedulerError::InvalidAdvanceTarget {
                source: NodeDisplay::from(map[X], reactor.get_meta(map[X])),
                target: NodeDisplay::from(map[Y], reactor.get_meta(map[Y])),
            }
        ))
    );

    Ok(())
}

/// That is, `X` has [Role::Regular] and can't issue [ScheduleDirective::Advance]
#[test]
fn non_regular_node_selection() -> Result {
    declare_tags!(X, Y);

    #[graph(X -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();
    mock_listeners!(reactor, no_op_listener, X, Y);

    reactor.start()?;

    assert_eq!(
        reactor.process(cmd::advance(map[X], map[Y])).err(),
        Some(ReactorError::Scheduler(
            SchedulerError::InvalidAdvanceSource {
                source: NodeDisplay::from(map[X], reactor.get_meta(map[X])),
                target: NodeDisplay::from(map[Y], reactor.get_meta(map[Y])),
            }
        ))
    );

    Ok(())
}

#[test]
fn nowhere_to_backtrack() -> Result {
    declare_tags!(X, Y);

    #[graph(X -> Y)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    mock_listeners!(reactor, no_op_listener, X, Y);

    reactor.start()?;

    assert_eq!(
        reactor.process(cmd::backtrack()).err(),
        Some(ReactorError::Scheduler(SchedulerError::NowhereToBacktrack))
    );

    Ok(())
}

#[test]
fn backtrack_into_parallel_group() -> Result {
    declare_tags!(A, X, Y);

    #[graph((X | Y) -> A)]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();
    mock_listeners!(reactor, no_op_listener, A, X, Y);

    reactor.start()?;
    reactor.process(cmd::resolve(map[X]))?;
    reactor.process(cmd::resolve(map[Y]))?;

    assert_eq!(
        reactor.process(cmd::backtrack()).err(),
        Some(ReactorError::Scheduler(
            SchedulerError::BacktrackToParallelBranch {
                from: NodeDisplay::from(map[A], reactor.get_meta(map[A])),
                to: NodeDisplay::from(map[Y], reactor.get_meta(map[Y])),
            }
        ))
    );

    Ok(())
}

#[test]
fn backtrack_from_parallel_group() -> Result {
    declare_tags!(A, X, Y);

    #[graph(A -> (X | Y))]
    struct Graph;

    let mut reactor = Reactor::from(Graph);
    let map = reactor.map_nodes();
    mock_listeners!(reactor, no_op_listener, A, X, Y);

    reactor.start()?;
    reactor.process(cmd::resolve(map[A]))?;

    assert_eq!(
        reactor.process(cmd::backtrack()).err(),
        Some(ReactorError::Scheduler(
            SchedulerError::BacktrackFromParallelBranch {
                from: NodeDisplay::from(map[Y], reactor.get_meta(map[Y])),
                to: NodeDisplay::from(map[X], reactor.get_meta(map[X])),
            }
        ))
    );

    Ok(())
}
