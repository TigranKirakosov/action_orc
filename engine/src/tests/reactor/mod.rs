use super::*;

mod selection_group;

#[test]
fn lifecycle_hooks() -> Result {
    declare_tags!(A, B, C);

    let g = orc!(
        A -> B -> C;
    );

    let mut reactor = Reactor::from(g);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| {
        ctx.record(id, status);
    });
    mock_listeners!(reactor, listener, A, B, C);

    assert!(logger.log.lock()?.is_empty());

    reactor.start()?;
    logger.check_log(vec![(map[A], NodeStatus::Started)]);

    reactor.process(cmd::resolve(map[A]))?;
    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[A], NodeStatus::Resolved(Resolution::Finished)),
        (map[B], NodeStatus::Started),
    ]);

    reactor.process(cmd::resolve(map[B]))?;
    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[A], NodeStatus::Resolved(Resolution::Finished)),
        (map[B], NodeStatus::Started),
        (map[B], NodeStatus::Resolved(Resolution::Finished)),
        (map[C], NodeStatus::Started),
    ]);

    reactor.process(cmd::resolve(map[C]))?;
    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[A], NodeStatus::Resolved(Resolution::Finished)),
        (map[B], NodeStatus::Started),
        (map[B], NodeStatus::Resolved(Resolution::Finished)),
        (map[C], NodeStatus::Started),
        (map[C], NodeStatus::Resolved(Resolution::Finished)),
    ]);

    Ok(())
}

#[test]
fn nested_pipeline_composition() -> Result {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    let mut room = IdentityGraph::new();
    add_nodes! {
          room,
          enter: Enter, exit: Exit,
    };
    room.add_edge(enter, exit);

    let mut combat = IdentityGraph::new();
    add_nodes! {
          combat,
          spawn: SpawnEnemies, fight: Fight,
    };
    combat.add_edge(spawn, fight);

    let mut loot = IdentityGraph::new();
    add_nodes! {
          loot,
          roll: RollLoot, pick: PickTreasure,
    };
    loot.add_edge(roll, pick);

    let combat_bounds = room.merge(&combat, vec![enter]);
    let _loot_bounds = room.merge(&loot, combat_bounds.sinks);

    let mut reactor = Reactor::from(room);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(
        reactor,
        no_op_listener,
        Enter,
        SpawnEnemies,
        Fight,
        RollLoot,
        PickTreasure
    );
    mock_listeners!(reactor, listener, Exit);

    reactor.start()?;
    for id in &tags![Enter, SpawnEnemies, Fight, RollLoot] {
        reactor.process(cmd::resolve(map[*id]))?;
    }

    assert!(logger.log.lock()?.is_empty(), "Exit blocked");

    reactor.process(cmd::resolve(map[PickTreasure]))?; // last task before Exit
    logger.check_log(vec![(map[Exit], NodeStatus::Started)]);

    Ok(())
}

#[test]
fn nested_pipeline_composition_macro() -> Result {
    declare_tags!(Enter, Exit);
    declare_tags!(SpawnEnemies, Fight);
    declare_tags!(RollLoot, PickTreasure);

    fn room(a: &IdentityGraph, b: &IdentityGraph) -> IdentityGraph {
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
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
        }
        _ => {}
    });
    mock_listeners!(
        reactor,
        no_op_listener,
        Enter,
        SpawnEnemies,
        Fight,
        RollLoot,
        PickTreasure
    );
    mock_listeners!(reactor, listener, Exit);

    reactor.start()?;
    for id in &tags![Enter, SpawnEnemies, Fight, RollLoot] {
        reactor.process(cmd::resolve(map[*id]))?;
    }

    assert!(logger.log.lock()?.is_empty(), "Exit blocked");

    reactor.process(cmd::resolve(map[PickTreasure]))?; // last task before Exit
    logger.check_log(vec![(map[Exit], NodeStatus::Started)]);

    Ok(())
}

/// Verifies [AsGraphProxy] and '@' expression prefix work in conjuction
#[test]
fn struct_expression() -> Result {
    declare_tags!(R, A, B, X, Y, X1, Y1);

    struct Race<'a> {
        a: &'a IdentityGraph,
        b: &'a IdentityGraph,
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

            GraphEntry::OwnedGraph(Box::new(g))
        }
    }

    fn composer(x: &IdentityGraph, y: &IdentityGraph) -> IdentityGraph {
        orc!(
            A -> @Race { a: x, b: y } -> B;
        )
    }

    let x = orc!(X -> X1;);
    let y = orc!(Y -> Y1;);

    let graph = composer(&x, &y);

    let mut reactor = Reactor::from(graph);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
            ctx.queue_cmd(cmd::resolve(id));
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, R, A, B, X, Y, X1, Y1);

    reactor.start()?;
    drain_commands(&mut reactor, logger.commands.clone())?;

    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[R], NodeStatus::Started),
        (map[X], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
        (map[X1], NodeStatus::Started),
        (map[Y1], NodeStatus::Started),
        (map[B], NodeStatus::Started),
    ]);

    Ok(())
}

/// An attribute-macro #[graph(...)] twin to [struct_expression]
#[test]
fn struct_expression_attribute_macro() -> Result {
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
    let expected_types = tags![R, A, B, X, Y, X1, Y1, O, K];
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

    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
            ctx.queue_cmd(cmd::resolve(id));
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, R, A, B, X, Y, X1, Y1, O, K);

    reactor.start()?;
    drain_commands(&mut reactor, logger.commands.clone())?;

    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[R], NodeStatus::Started),
        (map[X], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
        (map[X1], NodeStatus::Started),
        (map[Y1], NodeStatus::Started),
        (map[O], NodeStatus::Started),
        (map[K], NodeStatus::Started),
        (map[B], NodeStatus::Started),
    ]);

    Ok(())
}

/// Verifies #[graph(...)] works on unit structs
#[test]
fn unit_struct_expression_derive_macro() -> Result {
    declare_tags!(A, B, X, Y);

    #[graph(A -> (X | Y) -> B)]
    struct Unit;

    let mut reactor = Reactor::from(Unit);
    let map = reactor.map_nodes();

    let logger = TestLogger::new();
    let listener = logger.with_ctx(|id, status, ctx| match status {
        NodeStatus::Started => {
            ctx.record(id, status);
            ctx.queue_cmd(cmd::resolve(id));
        }
        _ => {}
    });
    mock_listeners!(reactor, listener, A, B, X, Y);

    reactor.start()?;
    drain_commands(&mut reactor, logger.commands.clone())?;

    logger.check_log(vec![
        (map[A], NodeStatus::Started),
        (map[X], NodeStatus::Started),
        (map[Y], NodeStatus::Started),
        (map[B], NodeStatus::Started),
    ]);

    Ok(())
}
