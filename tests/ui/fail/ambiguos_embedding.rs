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
    // ^ Graph<Ambiguous, Single>

    // It's ok to put it in the middle - neither `A` nor `C`
    // have any topology restrictions for ambiguous embeddings
    // at either of its edges
    let _ok = orc!(A -> (@both_single ? C));

    // `A` has a Selector role, `@amb` is Selection group member, which must have its entry strictly as `Single`
    let _err = orc!(A -> (@amb ? C));

    // Annotating with `Single` disambiguates `amb` into `Graph<Single, Single>`
    let _disambiguated = orc!(A -> (@[Single] amb ? C));
}
