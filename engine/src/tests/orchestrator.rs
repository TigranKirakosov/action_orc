use super::*;

#[test]
fn orchestrator_schedule_loop() -> Result {
    declare_tags!(A, B, C);

    #[graph(A -> B -> C)]
    struct Graph;

    let mut orchestrator = Orchestrator::new(
        Graph,
        OrcConfig {
            loop_schedule: true,
        },
    )?;

    let resolver = orchestrator.resolver();
    let map = orchestrator.reactor.map_nodes();

    orchestrator.start()?;

    let state = orchestrator.tick()?;
    assert!(
        state == State::Active,
        "Schedule should not be marked done while threads are processing"
    );

    let wave_1_events = orchestrator.drain_events();
    assert_eq!(wave_1_events, vec![(map[A], NodeStatus::Started),]);

    resolver.send(cmd::resolve(map[A]))?;

    let state = orchestrator.tick()?;
    assert!(state == State::Active);

    let wave_2_events = orchestrator.drain_events();
    assert_eq!(
        wave_2_events,
        vec![
            (map[A], NodeStatus::Resolved(Resolution::Finished)),
            (map[B], NodeStatus::Started),
        ]
    );

    resolver.send(cmd::resolve(map[B]))?;

    orchestrator.tick()?;
    let wave_3_events = orchestrator.drain_events();
    assert_eq!(
        wave_3_events,
        vec![
            (map[B], NodeStatus::Resolved(Resolution::Finished)),
            (map[C], NodeStatus::Started),
        ]
    );

    resolver.send(cmd::resolve(map[C]))?;

    orchestrator.tick()?;
    let wave_4_events = orchestrator.drain_events();
    assert_eq!(
        wave_4_events,
        vec![(map[C], NodeStatus::Resolved(Resolution::Finished)),]
    );

    let state = orchestrator.tick()?;
    assert!(
        state == State::Restarted,
        "Loop schedule resets instead of exiting execution"
    );

    let loop_reset_events = orchestrator.drain_events();
    assert_eq!(loop_reset_events, vec![(map[A], NodeStatus::Started),]);

    Ok(())
}

// F is blocked by [e]
// [e] is blocked by both [b] and [d]
// [d] is blocked by C
// Topological order:
// A, (B, C, D or C, D, B or C, B, D), E, F
#[test]
fn orchestrator_topological_correctness() -> Result {
    declare_tags!(A, B, C, D, E, F);

    #[graph(
        a: A -> (b: B | C -> d: D);
        ([b] | [d]) -> e: E;
        [e] -> F;
    )]
    struct Graph;

    let mut orchestrator = Orchestrator::new(
        Graph,
        OrcConfig {
            loop_schedule: true,
        },
    )?;

    let resolver = orchestrator.resolver();
    let map = orchestrator.reactor.map_nodes();

    orchestrator.start()?;

    let state = orchestrator.tick()?;
    assert!(state == State::Active);

    let events = orchestrator.drain_events();
    assert_eq!(events, vec![(map[A], NodeStatus::Started)]);

    resolver.send(cmd::resolve(map[A]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map[A], NodeStatus::Resolved(Resolution::Finished)),
            (map[B], NodeStatus::Started),
            (map[C], NodeStatus::Started),
        ]
    );

    // TEST NON-DETERMINISM: resolve the C -> D before B finishes
    resolver.send(cmd::resolve(map[C]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map[C], NodeStatus::Resolved(Resolution::Finished)),
            (map[D], NodeStatus::Started),
        ]
    );

    resolver.send(cmd::resolve(map[D]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![(map[D], NodeStatus::Resolved(Resolution::Finished)),]
    );

    resolver.send(cmd::resolve(map[B]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map[B], NodeStatus::Resolved(Resolution::Finished)),
            (map[E], NodeStatus::Started),
        ]
    );

    resolver.send(cmd::resolve(map[E]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map[E], NodeStatus::Resolved(Resolution::Finished)),
            (map[F], NodeStatus::Started),
        ]
    );

    resolver.send(cmd::resolve(map[F]))?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![(map[F], NodeStatus::Resolved(Resolution::Finished)),]
    );

    let state = orchestrator.tick()?;
    assert!(
        state == State::Restarted,
        "Loop schedule resets instead of exiting execution"
    );

    let events = orchestrator.drain_events();
    assert_eq!(events, vec![(map[A], NodeStatus::Started)]);

    Ok(())
}

#[test]
fn orchestrator_multiple_sources() -> Result {
    declare_tags!(SourceX, SourceY, BarrierNode);

    #[graph(
        SourceX -> b: BarrierNode;
        SourceY -> [b];
    )]
    struct ForkSourcesGraph;

    let mut orchestrator = Orchestrator::new(
        ForkSourcesGraph,
        OrcConfig {
            loop_schedule: false,
        },
    )?;
    let map = orchestrator.reactor.map_nodes();

    orchestrator.start()?;
    orchestrator.tick()?;

    let events = orchestrator.drain_events();
    assert_eq!(
        events,
        vec![
            (map[SourceX], NodeStatus::Started),
            (map[SourceY], NodeStatus::Started),
        ]
    );

    Ok(())
}

#[test]
fn orchestrator_no_loop_termination() -> Result {
    declare_tags!(X, Y);

    #[graph(X -> Y)]
    struct Graph;

    let mut orchestrator = Orchestrator::new(
        Graph,
        OrcConfig {
            loop_schedule: false,
        },
    )?;
    let resolver = orchestrator.resolver();
    let map = orchestrator.reactor.map_nodes();

    orchestrator.start()?;

    orchestrator.tick()?;
    orchestrator.drain_events();
    resolver.send(NodeCommand {
        schedule_directive: ScheduleDirective::Resolve { pivot: map[X] },
    })?;

    orchestrator.tick()?;
    orchestrator.drain_events();
    resolver.send(cmd::resolve(map[Y]))?;

    orchestrator.tick()?;
    let subsequent_events = orchestrator.drain_events();
    assert_eq!(
        subsequent_events,
        vec![(map[Y], NodeStatus::Resolved(Resolution::Finished))]
    );

    let state = orchestrator.tick()?;
    assert!(State::Ended == state);

    Ok(())
}
