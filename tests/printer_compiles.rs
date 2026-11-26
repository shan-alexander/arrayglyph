//! The desugared Rust is real Rust: each case pins the printer output and
//! then evaluates the exact same expression text.

use arrayglyph::prelude::*;
use arrayglyph::printer::glyph_to_rust;

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Asserts that the printer's output for `$glyph` is token-for-token the
/// Rust written after `=>`, then evaluates that Rust.
macro_rules! check {
    ($glyph:literal => $($rust:tt)*) => {{
        assert_eq!(
            squash(&glyph_to_rust($glyph).unwrap()),
            squash(stringify!($($rust)*)),
            "printer output for {}",
            $glyph
        );
        $($rust)*
    }};
}

#[test]
fn desugared_output_compiles_and_evaluates() {
    let a = Array::vector([1, 2, 3]);
    let b = Array::vector([4, 5, 6]);
    let m = Array::from_rows([[1, 2], [3, 4]]);

    let dot = check!("+/(a × b)" => a.zip_with(&b, Mul).reduce_axis::<1>(Add));
    assert_eq!(dot.item(), 32);

    // Prefix binds tighter: (+/a) × (+/b) = 6 × 15
    let prod =
        check!("+/a × +/b" => a.reduce_axis::<1>(Add).zip_with(&b.reduce_axis::<1>(Add), Mul));
    assert_eq!(prod.item(), 90);

    // Rust precedence, left to right: (2 × a) + b
    let fma = check!("2 × a + b" => Array::splat(2).zip_with(&a, Mul).zip_with(&b, Add));
    assert_eq!(fma.as_slice(), &[6, 9, 12]);

    // Left-associative subtraction: (b - a) - a
    let sub2 = check!("b - a - a" => b.zip_with(&a, Sub).zip_with(&a, Sub));
    assert_eq!(sub2.as_slice(), &[2, 1, 0]);

    let s = check!("-/a" => a.reduce_axis::<1>(Sub));
    assert_eq!(s.item(), -4);

    let run = check!("⌈\\(a - b)" => a.zip_with(&b, Sub).scan_axis::<1>(Max));
    assert_eq!(run.as_slice(), &[-3, -3, -3]);

    let ext = check!("2 × a" => Array::splat(2).zip_with(&a, Mul));
    assert_eq!(ext.as_slice(), &[2, 4, 6]);

    let lit = check!("1 2 3 + a" => Array::vector([1, 2, 3]).zip_with(&a, Add));
    assert_eq!(lit.as_slice(), &[2, 4, 6]);

    let sq = check!("m +.× ⍉m" => m.matmul(&m.transpose()));
    assert_eq!(sq.as_slice(), &[5, 11, 11, 25]);

    let rs = check!("2 3⍴⍳6" => iota::<6>().reshape::<2, 3>());
    assert_eq!(rs.row(1), &[3, 4, 5]);

    let colsum = check!("+/⍉m" => m.transpose().reduce_axis::<1>(Add));
    assert_eq!(colsum.as_slice(), &[4, 6]);

    let shp = check!("⍴m" => m.shape());
    assert_eq!(shp, (2, 2));

    let colsum2 = check!("(+/ ∘ ⍉) m" => compose(Reduce(Add).axis::<1>(), Transpose).call(&m));
    assert_eq!(colsum2, colsum);

    let three = check!("(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6"
        => compose(compose(Reduce(Add).axis::<1>(), Transpose), Reshape::<2, 3>).call(&iota::<6>()));
    assert_eq!(three.as_slice(), &[3, 5, 7]);

    let shifted = check!("(⌈\\ ∘ ⍉) m + 1"
        => compose(Scan(Max).axis::<1>(), Transpose).call(&m).zip_with(&Array::splat(1), Add));
    assert_eq!(shifted.as_slice(), &[2, 4, 3, 5]);

    // Axis 0 (`⌿`, `⍀`): down each column, the Rust default.
    let k = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    let cols = check!("+⌿k" => k.reduce(Add));
    assert_eq!(cols.as_slice(), &[5, 7, 9]);
    let rows = check!("+/k" => k.reduce_axis::<1>(Add));
    assert_eq!(rows.as_slice(), &[6, 15]);
    let total = check!("+/+⌿k" => k.reduce(Add).reduce_axis::<1>(Add));
    assert_eq!(total.item(), 21);
    let bracket = check!("+/[0] k" => k.reduce(Add));
    assert_eq!(bracket, cols);
    let run = check!("+⍀k" => k.scan(Add));
    assert_eq!(run.as_slice(), &[1, 2, 3, 5, 7, 9]);
    let halves = check!("+⌿k ÷ 2" => k.reduce(Add).zip_with(&Array::splat(2), Div));
    assert_eq!(halves.as_slice(), &[2, 3, 4]);
    let train = check!("(+/ ∘ +⌿) k" => compose(Reduce(Add).axis::<1>(), Reduce(Add)).call(&k));
    assert_eq!(train.item(), 21);
}
