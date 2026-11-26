//! Semantics of the array type and macros against hand-written loops.

use arrayglyph::prelude::*;

/// Reference left-to-right reduction written as an explicit loop.
fn ref_lfold(xs: &[i64], f: impl Fn(i64, i64) -> i64) -> i64 {
    let mut acc = xs[0];
    for &x in &xs[1..] {
        acc = f(acc, x);
    }
    acc
}

#[test]
fn reduce_is_left_to_right_for_all_ops() {
    let data = [7i64, 3, 9, 2, 5];
    let v = Array::vector(data);
    assert_eq!(reduce!(+, v).item(), ref_lfold(&data, |a, b| a + b));
    assert_eq!(reduce!(-, v).item(), ref_lfold(&data, |a, b| a - b));
    assert_eq!(reduce!(*, v).item(), ref_lfold(&data, |a, b| a * b));
    let pow2 = [64i64, 8, 4, 2]; // no zero divisors left-to-right
    assert_eq!(
        reduce!(/, Array::vector(pow2)).item(),
        ref_lfold(&pow2, |a, b| a / b)
    );
    assert_eq!(reduce!(max, v).item(), 9);
    assert_eq!(reduce!(min, v).item(), 2);
    // Left-fold reference values (APL's right fold would give 2, 4 and -2).
    assert_eq!(reduce!(-, Array::vector([1, 2, 3])).item(), -4); // (1-2)-3
    assert_eq!(reduce!(/, Array::vector([8, 4, 2])).item(), 1); // (8/4)/2
    assert_eq!(reduce!(-, Array::vector([1, 2, 3, 4])).item(), -8); // ((1-2)-3)-4
}

#[test]
fn scan_prefixes_match_reduce() {
    let data = [7i64, 3, 9, 2, 5];
    let v = Array::vector(data);
    let sub = scan!(-, v);
    let add = scan!(+, v);
    for k in 1..=data.len() {
        assert_eq!(sub[k - 1], ref_lfold(&data[..k], |a, b| a - b));
        assert_eq!(add[k - 1], ref_lfold(&data[..k], |a, b| a + b));
    }
    let pow2 = [64i64, 8, 4, 2];
    let div = scan!(/, Array::vector(pow2));
    assert_eq!(div.as_slice(), &[64, 8, 2, 1]);
    for k in 1..=pow2.len() {
        assert_eq!(div[k - 1], ref_lfold(&pow2[..k], |a, b| a / b));
    }
    assert_eq!(scan!(-, Array::vector([1, 2, 3])).as_slice(), &[1, -1, -4]);
    assert_eq!(scan!(min, v).as_slice(), &[7, 3, 3, 2, 2]);
}

#[test]
fn rank2_reduce_and_scan_along_last_axis() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    let r: Array<i32, 2, 1> = reduce!(+, m);
    assert_eq!(r.as_slice(), &[6, 15]);
    assert_eq!(scan!(+, m).as_slice(), &[1, 3, 6, 4, 9, 15]);
    // First axis via transpose.
    assert_eq!(reduce!(+, m.transpose()).as_slice(), &[5, 7, 9]);
    // Single-column reduce is the identity.
    assert_eq!(reduce!(-, r).as_slice(), &[6, 15]);
}

#[test]
fn macros_accept_references_and_expressions() {
    let m = Array::from_rows([[1, 2], [3, 4]]);
    let rm = &m;
    assert_eq!(reduce!(max, rm).as_slice(), &[2, 4]);
    assert_eq!(reduce!(+, &m + &m).as_slice(), &[6, 14]);
}

#[test]
fn floats_work() {
    let v = Array::vector([1.0, 2.0, 4.0]);
    assert_eq!(reduce!(/, v).item(), (1.0 / 2.0) / 4.0);
    assert_eq!(reduce!(max, v).item(), 4.0);
}

#[test]
fn elementwise_ops_match_loops() {
    let a = Array::from_fn(|r, c| (r * 3 + c) as i32 + 1);
    let b: Array<i32, 2, 3> = Array::from_fn(|r, c| (c as i32 + 1) * 10 - r as i32);
    let sum = &a + &b;
    let diff = &a - &b;
    let prod = a.clone() * b.clone();
    let quot = &b / &a;
    for r in 0..2 {
        for c in 0..3 {
            assert_eq!(sum[(r, c)], a[(r, c)] + b[(r, c)]);
            assert_eq!(diff[(r, c)], a[(r, c)] - b[(r, c)]);
            assert_eq!(prod[(r, c)], a[(r, c)] * b[(r, c)]);
            assert_eq!(quot[(r, c)], b[(r, c)] / a[(r, c)]);
        }
    }
    assert_eq!((-a.clone())[(1, 2)], -6);
}

#[test]
fn matmul_matches_triple_loop() {
    let a: Array<i64, 3, 4> = Array::from_fn(|r, c| (r * 4 + c) as i64 - 5);
    let b: Array<i64, 4, 2> = Array::from_fn(|r, c| (r as i64 + 1) * (c as i64 + 2));
    let c = a.matmul(&b);
    for i in 0..3 {
        for j in 0..2 {
            let mut s = 0;
            for k in 0..4 {
                s += a[(i, k)] * b[(k, j)];
            }
            assert_eq!(c[(i, j)], s);
        }
    }
}

#[test]
fn views_and_reshape() {
    let m = iota::<12>().reshape::<3, 4>();
    assert_eq!(m.row(2), &[8, 9, 10, 11]);
    assert_eq!(m.col(1).to_vec(), vec![1, 5, 9]);
    assert_eq!(
        m.col(3).iter().rev().copied().collect::<Vec<_>>(),
        vec![11, 7, 3]
    );
    assert_eq!(m.cols().len(), 4);
    assert_eq!(m.rows().len(), 3);
    let back: Vector<usize, 12> = m.clone().reshape();
    assert_eq!(back, iota::<12>());
    assert_eq!(m.transpose().transpose(), m);
    assert_eq!(m.transpose().shape(), (4, 3));
}

#[test]
fn from_vec_checks_length_at_runtime() {
    assert!(Array::<u8, 2, 3>::from_vec(vec![0; 6]).is_ok());
    let e = Array::<u8, 2, 3>::from_vec(vec![0; 5]).unwrap_err();
    assert_eq!(e.to_string(), "LENGTH ERROR: expected 6 elements, found 5");
}

#[test]
#[should_panic(expected = "out of bounds")]
fn index_out_of_bounds_panics() {
    let m = Array::from_rows([[1, 2], [3, 4]]);
    let _ = m[(2, 0)];
}

#[test]
fn display_is_apl_like() {
    let m = Array::from_rows([[1, -20], [300, 4]]);
    assert_eq!(m.to_string(), "  1 -20\n300   4");
}
