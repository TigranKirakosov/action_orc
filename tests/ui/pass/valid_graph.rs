use ::action_orc::*;

struct R;
struct A;
struct B;
struct C;
struct L;

// Define parametrized graph
#[graph( R -> (@a | @b) )]
#[params(a, b)]
struct Race;

// Define unit graph
#[graph( A -> B )]
struct Unit;

// Embed unit graph without '@' prefix
#[graph( Unit -> C )]
struct EmbedVariantOne;

// Embed unit graph wit '@' prefix
#[graph( @Unit -> C )]
struct EmbedVariantTwo;

// Embed compound graph strictly with '@' prefix
#[graph( @Race { a: x, b: y } -> C )]
#[params(x, y)]
struct EmbedVariantThree;

fn main() {
    // Implicit Leaf markers declaration
    let _graph = orc! {
        A -> B -> C;
    };

    // Make sharable refs so both _graph examples below could reuse them
    let combat = &orc!(C);
    let loot = &orc!(L);

    // Graph reference
    let _graph = orc! {
        A -> @combat -> @loot -> C;
    };

    // Complex graph expressions after @ prefix
    let _graph = orc! {
        A -> @Race { a: combat, b: loot } -> C;
    };
}
