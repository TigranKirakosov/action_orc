use std::str::FromStr;

use super::*;

#[test]
fn declaration() {
    let tokens = tokenize("a: A;");
    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn variable_binding() {
    let tokens = tokenize(
        "
        a: A;
        [a] -> B;
    ",
    );
    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn expression_binding() {
    let tokens = tokenize(
        "
        A -> @Struct { x, y } -> B;
    ",
    );
    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn sequence_group_syntax() {
    let tokens = tokenize(
        "
         X -> (A -> B, C -> D) -> Y;
     ",
    );

    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn fork_group_syntax() {
    let tokens = tokenize(
        "
         X -> (A -> B | C -> D) -> Y;
     ",
    );

    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn selection_group_syntax() {
    let tokens = tokenize(
        "
         X -> (A -> B ? C -> D) -> Y;
     ",
    );

    let ast = parse(tokens).unwrap();

    insta::assert_debug_snapshot!(ast);
}

#[test]
fn bound_annotation_syntax() {
    let tokens = tokenize("@[Fork; Single] fork_single;");
    let ast = parse(tokens).unwrap();
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn bound_annotation_splat_syntax() {
    let tokens = tokenize("@[Fork] both_fork;");
    let ast = parse(tokens).unwrap();
    insta::assert_debug_snapshot!(ast);
}

fn tokenize(i: &str) -> TokenStream2 {
    TokenStream2::from_str(i).unwrap()
}
