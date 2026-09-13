use ::action_orc::*;

struct A;
struct B;

fn main() {
    let _graph = orc! {
        a: A -> b: B;
        [b] -> [a];
    };

    let _graph = orc! {
        a: A -> B -> [a];
    };

    let combat = orc! { A; };

    let _graph = orc! {
        A -> combat -> B -> combat;
    };
}
