//! Explicit composition (`∘`) and function values.

use arrayglyph::prelude::*;

#[test]
fn compose_applies_right_then_left() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    // (+/ ∘ ⍉) m  ==  +/(⍉m)  (column sums)
    let f = compose(Reduce(Add), Transpose);
    assert_eq!(f.call(&m), m.transpose().reduce(Add));
    assert_eq!(f.call(&m).as_slice(), &[5, 7, 9]);
    // (⍉ ∘ +/) m  ==  ⍉(+/m)  (row sums as a row)
    let g = compose(Transpose, Reduce(Add));
    assert_eq!(g.call(&m).as_slice(), &[6, 15]);
    assert_eq!(g.call(&m).shape(), (1, 2));
}

#[test]
fn compose_is_associative() {
    let x = iota::<6>();
    let left = compose(compose(Reduce(Add), Transpose), Reshape::<2, 3>);
    let right = compose(Reduce(Add), compose(Transpose, Reshape::<2, 3>));
    assert_eq!(left.call(&x), right.call(&x));
}

#[test]
fn function_values_accept_owned_and_borrowed() {
    let v = Array::vector([3, 1, 4, 1, 5]);
    assert_eq!(Scan(Max).call(&v), Scan(Max).call(v.clone()));
    assert_eq!(Reduce(Sub).call(&v).item(), 3 - 1 - 4 - 1 - 5);
    assert_eq!(Shape.call(v.clone()), (1, 5));
    let owned = compose(Transpose, Reshape::<5, 1>).call(v.clone());
    assert_eq!(owned, v);
}

#[test]
fn composed_values_are_reusable_and_copy() {
    let col_max = compose(Reduce(Max), Transpose);
    let again = col_max; // Copy
    let a = Array::from_rows([[1, 9], [7, 2]]);
    let b = Array::from_rows([[0, 0], [5, 5]]);
    assert_eq!(col_max.call(&a).as_slice(), &[7, 9]);
    assert_eq!(again.call(&b).as_slice(), &[5, 5]);
}

#[test]
fn printer_output_for_trains_evaluates() {
    use arrayglyph::printer::glyph_to_rust;
    let m = Array::from_rows([[1, 2], [3, 4]]);
    assert_eq!(
        glyph_to_rust("(-/ ∘ ⍉) m").unwrap(),
        "compose(Reduce(Sub), Transpose).call(&m)"
    );
    // Left fold on each column: 1 - 3, 2 - 4
    assert_eq!(
        compose(Reduce(Sub), Transpose).call(&m).as_slice(),
        &[-2, -2]
    );
}
