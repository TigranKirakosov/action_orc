## DSL Concept
```rust
// Define parametrized dungeon scenario:
// 1. handle entering room
// 2. start 2 parallel tasks
// 3. start generic encounter
// 4. handle exiting room
fn dungeon_room(encounter: &Graph) -> Graph {
    orc! {
        // you can use both generic and pure labels
        Entering<Armed> -> (LoreNarration | Animations) -> encounter -> Exiting<UnArmed>;
    }
}

// Define custom encounters
fn goblins() -> Graph {
    orc! {
        SpawnGoblins -> Combat -> RollReward;
    }
}

fn merchant() -> Graph {
    orc! {
        OpenShopUI -> AwaitPlayerTransaction;
    }
}

// Compose
fn main() {
    let goblin_encounter = goblins();
    let merchant_encounter = merchant();

    let goblin_room = dungeon_room(&goblin_encounter);
    let merchant_room = dungeon_room(&merchant_encounter);

    let goblin_room_pipeline = Pipeline::from(goblin_room, |meta| /* map your ids to graph ids */);
    let merchant_room_pipeline = Pipeline::from(merchant_room, |meta| /* map your ids to graph ids */);
 
    goblin_room_pipeline.subscribe::<Combat>(|id, event| match event {
        Event::Started => { ... }, // receive control and eventually resolve a task
        Event::Resolved => { ... }
    });

    goblin_room_pipeline.init();
}
```

# Directives

## race
If `a` finishes first, `b` and `c` are discarded while `d` gets started:
- `race(a | b | c) -> d`

## fallback
Mark fallback node (`f`) from where to recover when a downstream has failed; and provide retry count:
- `fallback(f, retry_count) -> (a | b | c) -> (d | e)`
