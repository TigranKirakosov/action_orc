use ::action_orc::*;

struct A;
struct B;
struct C;

fn main() {
    let both_single = orc!(
        A -> B -> C;
    );

    // Unannotated edge embedding is `Ambiguous`
    let amb = orc!(@both_single -> C);
    // ^ Graph<Ambiguous, Join>

    // It's ok to put it in the middle - neither `A` nor `C`
    // have any topology restrictions for ambiguous embeddings
    // at either of its edges
    let _ok = orc!(A -> (@both_single ? C));

    // `A` has a Selector role, `@amb` is Selection group member, which must have its entry strictly as `Join`
    let _err = orc!(A -> (@amb ? C));

    // Annotating with `Join` disambiguates `amb` into `Graph<Join, Join>`
    let _disambiguated = orc!(A -> (@[Join] amb ? C));
}
