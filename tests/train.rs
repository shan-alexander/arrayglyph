//! Explicit composition (`∘`) and function values.

use arrayglyph::prelude::*;

#[test]
fn compose_applies_right_then_left() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    // (+/ ∘ ⍉) m  ==  +/(⍉m)  (column sums)
    let f = compose(Reduce(Add).axis::<1>(), Transpose);
    assert_eq!(f.call(&m), m.transpose().reduce_axis::<1>(Add));
    assert_eq!(f.call(&m).as_slice(), &[5, 7, 9]);
    // (⍉ ∘ +/) m  ==  ⍉(+/m)  (row sums as a row)
    let g = compose(Transpose, Reduce(Add).axis::<1>());
    assert_eq!(g.call(&m).as_slice(), &[6, 15]);
    assert_eq!(g.call(&m).shape(), (1, 2));
}

#[test]
fn compose_is_associative() {
    let x = iota::<6>();
    let left = compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>);
    let right = compose(Reduce(Add).axis::<1>(), compose(Transpose, Reshape::<2, 3>));
    assert_eq!(left.call(&x), right.call(&x));
}

#[test]
fn function_values_accept_owned_and_borrowed() {
    let v = Array::vector([3, 1, 4, 1, 5]);
    assert_eq!(
        Scan(Max).axis::<1>().call(&v),
        Scan(Max).axis::<1>().call(v.clone())
    );
    assert_eq!(Reduce(Sub).axis::<1>().call(&v).item(), 3 - 1 - 4 - 1 - 5);
    assert_eq!(Shape.call(v.clone()), (1, 5));
    let owned = compose(Transpose, Reshape::<5, 1>).call(v.clone());
    assert_eq!(owned, v);
}

#[test]
fn composed_values_are_reusable_and_copy() {
    let col_max = compose(Reduce(Max).axis::<1>(), Transpose);
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
        "compose(Reduce(Sub).axis::<1>(), Transpose).call(&m)"
    );
    // Left fold on each column: 1 - 3, 2 - 4
    assert_eq!(
        compose(Reduce(Sub).axis::<1>(), Transpose)
            .call(&m)
            .as_slice(),
        &[-2, -2]
    );
}

#[test]
fn axis_function_values_match_methods() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    // `Reduce(F)` is axis 0 (`f⌿`), `.axis::<1>()` is `f/`.
    assert_eq!(Reduce(Add).call(&m), m.reduce(Add));
    assert_eq!(Reduce(Add).axis::<0>().call(&m), m.reduce(Add));
    assert_eq!(Reduce(Add).axis::<1>().call(&m), m.reduce_axis::<1>(Add));
    assert_eq!(Scan(Max).call(&m), m.scan(Max));
    assert_eq!(Scan(Max).axis::<1>().call(m.clone()), m.scan_axis::<1>(Max));
    // (+/ ∘ +⌿) m: grand total as a train.
    let total = compose(Reduce(Add).axis::<1>(), Reduce(Add));
    assert_eq!(total.call(&m).item(), 21);
}

#[test]
fn flat_train_macro_matches_nested_compose() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    let nested = compose(Reduce(Add).axis::<1>(), Transpose);
    let flat = train![reduce(+[1]), transpose];
    assert_eq!(flat.call(&m), nested.call(&m));

    // Rightmost runs first: reshape, then transpose, then sum rows.
    let three = train![reduce(+[1]), transpose, reshape(2, 3)];
    assert_eq!(three.call(&iota::<6>()).as_slice(), &[3, 5, 7]);
    let by_hand = iota::<6>().reshape::<2, 3>().transpose().sum::<1>();
    assert_eq!(three.call(iota::<6>()), by_hand);

    // Scans, default axis, shape, and parenthesised items.
    assert_eq!(train![scan(*[1])].call(&m), m.cumprod::<1>());
    assert_eq!(train![reduce(max)].call(&m), m.max::<0>());
    assert_eq!(train![shape, transpose].call(&m), (3, 2));
    let inner = train![transpose];
    assert_eq!(
        train![reduce(+[1]), (inner)].call(&m),
        m.sum::<0>().transpose()
    );
    // Trailing comma is fine.
    assert_eq!(train![transpose, transpose,].call(&m), m);
}
