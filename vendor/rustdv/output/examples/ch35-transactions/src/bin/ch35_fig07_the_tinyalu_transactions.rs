// Rust for RTL Verification — Chapter 35, Figure 7
// "The TinyALU transactions, final form"
// Run with: cargo run --bin ch35_fig07_the_tinyalu_transactions
//
// Expected output:
//   cmd:        A: a5 Mul B: 75
//   debug:      AluCommand { a: 165, b: 117, op: Mul }
//   clone == cmd? true
//   tweaked == cmd? false
//   ops covered: 4 of 4

use std::collections::HashSet;
use std::fmt;

// Everything the chapter covered, applied. These three definitions replace
// the `(u8, u8, Ops)` tuples the testbench has been passing since Chapter 19,
// and they carry every remaining chapter of the book.

// `Eq` and `Hash` because coverage puts an op in a `HashSet` (Figure 3).
// `Copy` because an op is one byte and copying it is free — the one place in
// the testbench where `Copy` is the right answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

impl Ops {
    pub const ALL: [Ops; 4] = [Ops::Add, Ops::And, Ops::Xor, Ops::Mul];
}

// A command is plain data: no base class, no framework trait, nothing to
// extend. `Clone` is `do_copy()`, `PartialEq` is `do_compare()`, `Debug` is
// the field dump — each generated from the field list, and each updating
// itself when you add a field.
//
// Not `Copy`: a transaction is something you hand over, and Chapter 31 showed
// what goes wrong when a `Copy` stand-in lets the wrong code compile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

// `Display` is the one you write, because only you know that a command reads
// best as operand-operator-operand.
impl fmt::Display for AluCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A: {:02x} {:?} B: {:02x}", self.a, self.op, self.b)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AluResult {
    pub result: u16,
}

fn main() {
    let cmd = AluCommand { a: 0xA5, b: 0x75, op: Ops::Mul };

    println!("cmd:        {cmd}");
    println!("debug:      {cmd:?}");

    let copy = cmd.clone();
    println!("clone == cmd? {}", copy == cmd);

    let mut tweaked = cmd.clone();
    tweaked.a = 0;
    println!("tweaked == cmd? {}", tweaked == cmd);

    // And the coverage use that made `Eq` and `Hash` necessary.
    let mut covered: HashSet<Ops> = HashSet::new();
    for op in Ops::ALL {
        covered.insert(op);
    }
    println!("ops covered: {} of {}", covered.len(), Ops::ALL.len());

    let _ = AluResult { result: 0x4B69 };
}
