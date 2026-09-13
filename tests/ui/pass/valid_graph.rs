use ::action_orc::*;

struct R;
struct A;
struct B;
struct C;
struct L;

#[derive(Graph)]
#[orc( R -> (@a | @b) )]
pub struct Race<'a> {
    a: &'a Graph,
    b: &'a Graph,
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

// #[derive(Graph)] under the hood
// impl<'a> AsGraphEntryProxy<'a> for Race<'a> {
//     fn as_entry_proxy(self) -> GraphEntry<'a> {
//         let Self { a, b } = self;

//         GraphEntry::OwnedGraph(orc! {
//             R -> ( @a | @b )
//         })
//     }
// }
