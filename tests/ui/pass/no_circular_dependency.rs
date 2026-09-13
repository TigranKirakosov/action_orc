use ::action_orc::*;

struct A;
struct B;
struct C;

/// foo and bar are unique instances, hence no circular dependency
fn main() {
    let _graph = orc! {
        foo: A -> B -> bar: A -> C;
    };
}
