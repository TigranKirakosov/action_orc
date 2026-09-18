use ::action_orc::*;

struct A;
struct B;
struct C;
struct X;
struct Y;
struct Z;

#[graph(X -> (A ? (B | C)) -> Y)]
struct Graph;

fn main() {}
