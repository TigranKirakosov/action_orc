use ::action_orc::*;

struct Enter;
struct Exit;
struct TaskA;
struct TaskB;
struct TaskC;

fn combat_fn() -> Graph {
    orc! {
        TaskA -> TaskB;
    }
}

fn loot_fn() -> Graph {
    orc! {
        TaskC;
    }
}

/// Use refs (&) for combat and loot graphs so that they
/// could be used by subsequent graph declarations
fn main() {
    let (combat, loot) = (&combat_fn(), &loot_fn());

    let _linear_composed = orc! {
        Enter -> #[combat] -> #[loot] -> Exit;
    };

    let _parallel_composed = orc! {
        Enter -> ( TaskA | #[combat] | TaskB ) -> Exit;
    };

    let _start_embedded = orc! {
        #[combat] -> Exit;
    };
}
