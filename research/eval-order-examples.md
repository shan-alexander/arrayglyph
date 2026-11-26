# Evaluation order: right-to-left (APL) vs left-to-right (Rust-style)

Same glyph text, two readings. "Rust-style" means left-associative with precedence (× before +, reduce binds like a function call).

## 1. Mixed scalar ops: `2 × 3 + 4`

| | Steps | Result |
|---|---|---|
| Right-to-left | 3 + 4 = 7, then 2 × 7 | **14** |
| Rust-style | 2 × 3 = 6, then 6 + 4 | **10** |

## 2. Non-associative reduce: `-/ 1 2 3 4`

| | Steps | Result |
|---|---|---|
| Right-to-left (right fold) | 3 − 4 = −1, 2 − (−1) = 3, 1 − 3 | **−2** |
| Left fold | 1 − 2 = −1, −1 − 3 = −4, −4 − 4 | **−8** |

(Same split for `÷/ 8 4 2`: right-to-left 8 ÷ (4 ÷ 2) = **4**, left fold (8 ÷ 4) ÷ 2 = **1**.)

## 3. Reduce scope: `+/ 1 2 3 × 4 5 6`

| | Steps | Result |
|---|---|---|
| Right-to-left | 1 2 3 × 4 5 6 = 4 10 18, then +/ | **32** (sum of products) |
| Rust-style | +/ 1 2 3 = 6, then 6 × 4 5 6 | **24 30 36** |

## Upshot

The two directions diverge whenever a non-associative op (−, ÷) or mixed ops appear, and in how far a reduce reaches: right-to-left gives the reduce everything to its right, so different values and even different result shapes (a scalar vs a vector).
