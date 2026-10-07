//! `#[derive(Eq)]` on a transaction with a float field must not compile.
//!
//! Chapter 35's Figure 3 shows a reader why `PartialEq` is *partial*: `f64`
//! has NaN, NaN != NaN, and so floats are not an equivalence relation. Rust
//! encodes that by giving `f64` a `PartialEq` and no `Eq`, and a transaction
//! that derives `Eq` while carrying one is rejected.
//!
//! The chapter names the error. Naming it in prose and checking it are two
//! different things, and only one of them notices when the compiler's
//! wording or code changes.

// A measurement transaction, of the kind an analog testbench would collect.
#[derive(PartialEq, Eq)]
pub struct Sample {
    pub cycle: u32,
    pub voltage: f64,
}
