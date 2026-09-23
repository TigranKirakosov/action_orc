use ::action_orc::*;

struct R;
struct A;
struct B;
struct C;

#[graph(R -> (@a | @b))]
struct Race<'a> {
    a: &'a Graph<Single, Single>,
    b: &'a Graph<Single, Single>,
}

// Compound expression `Race { a: x, b: y }` without the '@' prefix is invalid:
// - expressions in the DSL require `@`
// - inline compound forms must be `@`-prefixed
#[graph(Race { a: x, b: y } -> C)]
struct EmbeddedRace<'a> {
    x: &'a Graph<Single, Single>,
    y: &'a Graph<Single, Single>,
}

fn main() {
    let _graph_blueprint = EmbeddedRace {
        x: &orc!(A),
        y: &orc!(B),
    };
}
