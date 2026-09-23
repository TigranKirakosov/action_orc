use ::action_orc::*;

struct R;
struct A;
struct B;
struct C;
struct L;

// Parametrized graph: fields declare the sub-graphs referenced after `@`.
#[graph( R -> (@a | @b) )]
struct Race<'a> {
    a: &'a Graph<Single, Single>,
    b: &'a Graph<Single, Single>,
}

// Unit graph: a #[graph]-decorated unit struct may be embedded without `@`.
#[graph( A -> B )]
struct Unit;

// Embed unit graph without '@' prefix (bare unit struct with #[graph]).
#[graph( Unit -> C )]
struct EmbedVariantOne;

// Embed compound graph strictly with '@' prefix and struct expression.
#[graph( @Race { a: x, b: y } -> C )]
struct EmbedVariantThree<'a> {
    x: &'a Graph<Single, Single>,
    y: &'a Graph<Single, Single>,
}

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
