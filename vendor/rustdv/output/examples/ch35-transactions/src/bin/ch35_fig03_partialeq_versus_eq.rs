// Rust for RTL Verification — Chapter 35, Figure 3
// "Why equality comes in two traits"
// Run with: cargo run --bin ch35_fig03_partialeq_versus_eq
//
// A `Measurement` cannot be a coverage key, and this file is the reason.
// Uncomment the `#[derive(Eq)]` on `Measurement` and the compiler says:
//   error[E0277]: the trait bound `f64: Eq` is not satisfied
//
// Expected output:
//   a measured delay is not equal to itself: NaN == NaN? false
//   ops seen: 4

use std::collections::HashSet;

// `PartialEq` promises symmetry and transitivity: if a == b then b == a, and
// if a == b and b == c then a == c. It does **not** promise that a == a.
//
// That sounds like hair-splitting until you meet a float. IEEE 754 says NaN
// is equal to nothing, including itself, so `f64` can only ever be
// `PartialEq`. Had Rust demanded reflexivity from every equality, floats
// could not have one at all.
#[derive(Debug, PartialEq)]
// #[derive(Eq)]  // <-- will not compile: f64 is not Eq
struct Measurement {
    delay_ns: f64,
}

// `Eq` adds the missing promise — every value equals itself — and carries no
// methods. It is a claim, checked by the compiler, that your equality is a
// real equivalence relation.
//
// Things that look values up by key need that claim, because a key that does
// not equal itself could never be found again. `HashSet` and `HashMap` both
// require it, which is why every coverage bin in this book derives `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

fn main() {
    let m = Measurement { delay_ns: f64::NAN };
    println!("a measured delay is not equal to itself: NaN == NaN? {}", m == m);

    // `Ops` is `Eq`, so it can be a coverage key.
    let mut seen: HashSet<Ops> = HashSet::new();
    for op in [Ops::Add, Ops::Mul, Ops::And, Ops::Xor, Ops::Add] {
        seen.insert(op); // the last one is already in the set
    }
    println!("ops seen: {}", seen.len());
}
