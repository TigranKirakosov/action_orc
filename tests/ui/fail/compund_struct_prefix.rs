use ::action_orc::*;

struct R;
struct A;
struct B;
struct C;

#[graph( R -> (@a | @b) )]
#[params(a, b)]
struct Race;

// Compound graph without '@' prefix should fail validation
#[graph( Race { a: x, b: y } -> C )]
#[params(x, y)]
struct EmbeddedRace;

fn main() {
    let _graph_blueprint = EmbeddedRace {
        x: &orc!(A),
        y: &orc!(B),
    };
}
