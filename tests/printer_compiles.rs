//! The desugared Rust is real Rust: each case pins the printer output (in
//! method or macro style) and then evaluates the exact same expression text.

use arrayglyph::prelude::*;
use arrayglyph::printer::{Style, glyph_to_rust, glyph_to_rust_with};

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Asserts that the printer's method-style output for `$glyph` is
/// token-for-token the Rust written after `=>`, then evaluates that Rust.
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

/// The same, for macro-style output.
macro_rules! check_macro {
    ($glyph:literal => $($rust:tt)*) => {{
        assert_eq!(
            squash(&glyph_to_rust_with($glyph, Style::Macro).unwrap()),
            squash(stringify!($($rust)*)),
            "macro-style printer output for {}",
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

    let dot = check!("+/(a × b)" => a.zip_with(&b, Mul).sum::<1>());
    assert_eq!(dot.item(), 32);

    // Prefix binds tighter: (+/a) × (+/b) = 6 × 15
    let prod = check!("+/a × +/b" => a.sum::<1>().zip_with(&b.sum::<1>(), Mul));
    assert_eq!(prod.item(), 90);

    // Rust precedence, left to right: (2 × a) + b
    let fma = check!("2 × a + b" => Array::splat(2).zip_with(&a, Mul).zip_with(&b, Add));
    assert_eq!(fma.as_slice(), &[6, 9, 12]);

    // Left-associative subtraction: (b - a) - a
    let sub2 = check!("b - a - a" => b.zip_with(&a, Sub).zip_with(&a, Sub));
    assert_eq!(sub2.as_slice(), &[2, 1, 0]);

    // `-` and `÷` have no named method.
    let s = check!("-/a" => a.reduce_axis::<1>(Sub));
    assert_eq!(s.item(), -4);

    let run = check!("⌈\\(a - b)" => a.zip_with(&b, Sub).cummax::<1>());
    assert_eq!(run.as_slice(), &[-3, -3, -3]);

    let ext = check!("2 × a" => Array::splat(2).zip_with(&a, Mul));
    assert_eq!(ext.as_slice(), &[2, 4, 6]);

    let lit = check!("1 2 3 + a" => Array::vector([1, 2, 3]).zip_with(&a, Add));
    assert_eq!(lit.as_slice(), &[2, 4, 6]);

    let sq = check!("m +.× ⍉m" => m.matmul(&m.transpose()));
    assert_eq!(sq.as_slice(), &[5, 11, 11, 25]);

    let rs = check!("2 3⍴⍳6" => iota::<6>().reshape::<2, 3>());
    assert_eq!(rs.row(1), &[3, 4, 5]);

    let colsum = check!("+/⍉m" => m.transpose().sum::<1>());
    assert_eq!(colsum.as_slice(), &[4, 6]);

    let shp = check!("⍴m" => m.shape());
    assert_eq!(shp, (2, 2));

    let colsum2 = check!("(+/ ∘ ⍉) m" => train![reduce(+[1]), transpose].call(&m));
    assert_eq!(colsum2, colsum);

    let three = check!("(+/ ∘ ⍉ ∘ 2 3⍴) ⍳6"
        => train![reduce(+[1]), transpose, reshape(2, 3)].call(&iota::<6>()));
    assert_eq!(three.as_slice(), &[3, 5, 7]);

    let shifted = check!("(⌈\\ ∘ ⍉) m + 1"
        => train![scan(max[1]), transpose].call(&m).zip_with(&Array::splat(1), Add));
    assert_eq!(shifted.as_slice(), &[2, 4, 3, 5]);

    // Axis 0 (`⌿`, `⍀`): down each column.
    let k = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    let cols = check!("+⌿k" => k.sum::<0>());
    assert_eq!(cols.as_slice(), &[5, 7, 9]);
    let rows = check!("+/k" => k.sum::<1>());
    assert_eq!(rows.as_slice(), &[6, 15]);
    let total = check!("+/+⌿k" => k.sum::<0>().sum::<1>());
    assert_eq!(total.item(), 21);
    let bracket = check!("+/[0] k" => k.sum::<0>());
    assert_eq!(bracket, cols);
    let run = check!("+⍀k" => k.cumsum::<0>());
    assert_eq!(run.as_slice(), &[1, 2, 3, 5, 7, 9]);
    let p = check!("×⌿k" => k.prod::<0>());
    assert_eq!(p.as_slice(), &[4, 10, 18]);
    let mn = check!("⌊/k" => k.min::<1>());
    assert_eq!(mn.as_slice(), &[1, 4]);
    let cp = check!("×\\k" => k.cumprod::<1>());
    assert_eq!(cp.as_slice(), &[1, 2, 6, 4, 20, 120]);
    let cm = check!("⌊⍀k" => k.cummin::<0>());
    assert_eq!(cm.as_slice(), &[1, 2, 3, 1, 2, 3]);
    let d = check!("÷⍀k" => k.scan_axis::<0>(Div));
    assert_eq!(d.as_slice(), &[1, 2, 3, 0, 0, 0]);
    let halves = check!("+⌿k ÷ 2" => k.sum::<0>().zip_with(&Array::splat(2), Div));
    assert_eq!(halves.as_slice(), &[2, 3, 4]);
    let train = check!("(+/ ∘ +⌿) k" => train![reduce(+[1]), reduce(+[0])].call(&k));
    assert_eq!(train.item(), 21);
}

#[test]
fn macro_style_output_compiles_and_evaluates() {
    let a = Array::vector([1, 2, 3]);
    let b = Array::vector([4, 5, 6]);
    let k = Array::from_rows([[1, 2, 3], [4, 5, 6]]);

    let dot = check_macro!("+/(a × b)" => reduce!(+[1], a.zip_with(&b, Mul)));
    assert_eq!(dot.item(), 32);
    let prod = check_macro!("+/a × +/b" => reduce!(+[1], a).zip_with(&reduce!(+[1], b), Mul));
    assert_eq!(prod.item(), 90);
    let s = check_macro!("-/a" => reduce!(-[1], a));
    assert_eq!(s.item(), -4);
    let q = check_macro!("÷/b" => reduce!(/[1], b));
    assert_eq!(q.item(), 0);
    let cols = check_macro!("+⌿k" => reduce!(+[0], k));
    assert_eq!(cols.as_slice(), &[5, 7, 9]);
    let total = check_macro!("+/+⌿k" => reduce!(+[1], reduce!(+[0], k)));
    assert_eq!(total.item(), 21);
    let mx = check_macro!("⌈⌿k" => reduce!(max[0], k));
    assert_eq!(mx.as_slice(), &[4, 5, 6]);
    let mn = check_macro!("⌊/k" => reduce!(min[1], k));
    assert_eq!(mn.as_slice(), &[1, 4]);
    let run = check_macro!("+⍀k" => scan!(+[0], k));
    assert_eq!(run.as_slice(), &[1, 2, 3, 5, 7, 9]);
    let cp = check_macro!("×\\k" => scan!(*[1], k));
    assert_eq!(cp.as_slice(), &[1, 2, 6, 4, 20, 120]);
    let cm = check_macro!("⌈\\(a - b)" => scan!(max[1], a.zip_with(&b, Sub)));
    assert_eq!(cm.as_slice(), &[-3, -3, -3]);
    let halves = check_macro!("+⌿k ÷ 2" => reduce!(+[0], k).zip_with(&Array::splat(2), Div));
    assert_eq!(halves.as_slice(), &[2, 3, 4]);
    let t = check_macro!("(+/ ∘ ⍉) k" => train![reduce(+[1]), transpose].call(&k));
    assert_eq!(t.as_slice(), &[5, 7, 9]);
    let nested =
        check_macro!("+/(⍉ ∘ ⍉) k" => reduce!(+[1], train![transpose, transpose].call(&k)));
    assert_eq!(nested.as_slice(), &[6, 15]);
}
