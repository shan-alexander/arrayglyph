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
    assert_eq!(
        reduce!(+, v, axis = 1).item(),
        ref_lfold(&data, |a, b| a + b)
    );
    assert_eq!(
        reduce!(-, v, axis = 1).item(),
        ref_lfold(&data, |a, b| a - b)
    );
    assert_eq!(
        reduce!(*, v, axis = 1).item(),
        ref_lfold(&data, |a, b| a * b)
    );
    let pow2 = [64i64, 8, 4, 2]; // no zero divisors left-to-right
    assert_eq!(
        reduce!(/, Array::vector(pow2), axis = 1).item(),
        ref_lfold(&pow2, |a, b| a / b)
    );
    assert_eq!(reduce!(max, v, axis = 1).item(), 9);
    assert_eq!(reduce!(min, v, axis = 1).item(), 2);
    // Left-fold reference values (APL's right fold would give 2, 4 and -2).
    assert_eq!(reduce!(-, Array::vector([1, 2, 3]), axis = 1).item(), -4); // (1-2)-3
    assert_eq!(reduce!(/, Array::vector([8, 4, 2]), axis = 1).item(), 1); // (8/4)/2
    assert_eq!(reduce!(-, Array::vector([1, 2, 3, 4]), axis = 1).item(), -8); // ((1-2)-3)-4
}

#[test]
fn scan_prefixes_match_reduce() {
    let data = [7i64, 3, 9, 2, 5];
    let v = Array::vector(data);
    let sub = scan!(-, v, axis = 1);
    let add = scan!(+, v, axis = 1);
    for k in 1..=data.len() {
        assert_eq!(sub[k - 1], ref_lfold(&data[..k], |a, b| a - b));
        assert_eq!(add[k - 1], ref_lfold(&data[..k], |a, b| a + b));
    }
    let pow2 = [64i64, 8, 4, 2];
    let div = scan!(/, Array::vector(pow2), axis = 1);
    assert_eq!(div.as_slice(), &[64, 8, 2, 1]);
    for k in 1..=pow2.len() {
        assert_eq!(div[k - 1], ref_lfold(&pow2[..k], |a, b| a / b));
    }
    assert_eq!(
        scan!(-, Array::vector([1, 2, 3]), axis = 1).as_slice(),
        &[1, -1, -4]
    );
    assert_eq!(scan!(min, v, axis = 1).as_slice(), &[7, 3, 3, 2, 2]);
}

#[test]
fn rank2_reduce_and_scan_along_each_axis() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    // axis 0 (the default): down each column, 2×3 -> 1×3.
    let c: Array<i32, 1, 3> = reduce!(+, m);
    assert_eq!(c.as_slice(), &[5, 7, 9]);
    assert_eq!(reduce!(+, m, axis = 0), c);
    assert_eq!(m.reduce_axis::<0>(Add), c);
    assert_eq!(scan!(+, m).as_slice(), &[1, 2, 3, 5, 7, 9]);
    assert_eq!(scan!(+, m, axis = 0), scan!(+, m));
    // axis 1: along each row, 2×3 -> 2×1.
    let r: Array<i32, 2, 1> = reduce!(+, m, axis = 1);
    assert_eq!(r.as_slice(), &[6, 15]);
    assert_eq!(scan!(+, m, axis = 1).as_slice(), &[1, 3, 6, 4, 9, 15]);
    // Transposing swaps the axes.
    assert_eq!(
        reduce!(+, m.transpose(), axis = 1).into_vec(),
        c.clone().into_vec()
    );
    assert_eq!(reduce!(+, m.transpose()).into_vec(), r.clone().into_vec());
    // Both axes is two calls, in either order.
    assert_eq!(reduce!(+, c, axis = 1).item(), 21);
    assert_eq!(reduce!(+, r).item(), 21);
}

#[test]
fn axis0_folds_left_down_columns() {
    let m = Array::from_rows([[10_i64, 1], [3, 2], [4, 8]]);
    // (10 - 3) - 4 = 3 and (1 - 2) - 8 = -9
    assert_eq!(reduce!(-, m).as_slice(), &[3, -9]);
    assert_eq!(scan!(-, m).as_slice(), &[10, 1, 7, -1, 3, -9]);
    assert_eq!(reduce!(max, m).as_slice(), &[10, 8]);
    assert_eq!(scan!(min, m).as_slice(), &[10, 1, 3, 1, 3, 1]);
    // Every op agrees with a reference left fold over each column.
    let cols: Vec<Vec<i64>> = (0..2).map(|j| m.col(j).iter().copied().collect()).collect();
    let mul = reduce!(*, m);
    for (j, col) in cols.iter().enumerate() {
        assert_eq!(mul[(0, j)], ref_lfold(col, |a, b| a * b));
    }
}

#[test]
fn macros_accept_references_and_expressions() {
    let m = Array::from_rows([[1, 2], [3, 4]]);
    let rm = &m;
    assert_eq!(reduce!(max, rm, axis = 1).as_slice(), &[2, 4]);
    assert_eq!(reduce!(+, &m + &m, axis = 1).as_slice(), &[6, 14]);
}

#[test]
fn floats_work() {
    let v = Array::vector([1.0, 2.0, 4.0]);
    assert_eq!(reduce!(/, v, axis = 1).item(), (1.0 / 2.0) / 4.0);
    assert_eq!(reduce!(max, v, axis = 1).item(), 4.0);
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

#[test]
fn display_honours_precision_and_width() {
    let m = Array::from_rows([[1.5, -2.0], [10.125, 0.0]]);
    assert_eq!(format!("{m:.2}"), " 1.50 -2.00\n10.12  0.00");
    assert_eq!(format!("{m:6.1}"), "   1.5   -2.0\n  10.1    0.0");
    // Without a precision, elements print as Display does.
    assert_eq!(m.to_string(), "   1.5 -2\n10.125  0");
}

#[test]
fn broadcast_stretches_length_one_axes() {
    let m = Array::from_rows([[1, 2, 3], [4, 5, 6]]);
    let row = Array::vector([10, 20, 30]);
    let col: Array<i32, 2, 1> = Array::from_rows([[100], [200]]);
    assert_eq!(
        m.zip_with(&row.broadcast(), Add).as_slice(),
        &[11, 22, 33, 14, 25, 36]
    );
    assert_eq!(
        m.zip_with(&col.broadcast(), Add).as_slice(),
        &[101, 102, 103, 204, 205, 206]
    );
    // A 1×1 broadcasts to anything; a matching shape is a copy.
    let one = Array::vector([7]);
    assert_eq!(one.broadcast::<2, 2>().as_slice(), &[7, 7, 7, 7]);
    assert_eq!(m.broadcast::<2, 3>(), m);
    // Centre each column on its own mean (the showcase's normalisation).
    let f = Array::from_rows([[1.0, 10.0], [3.0, 30.0]]);
    let col_means = f.reduce(Add).map(|x| x / 2.0);
    let centred = f.zip_with(&col_means.broadcast(), Sub);
    assert_eq!(centred.as_slice(), &[-1.0, -10.0, 1.0, 10.0]);
}
