// Rust for RTL Verification — Chapter 10, Figure 6
// "Implementing Display by hand — the __str__ of Rust"
// Run with: cargo run --bin ch10_fig06_implementing_display_hand_str
//
// Expected output:
//   cmd: a=0xaa b=0x55 op=Xor


// ---- Context from Chapter 10, Figure 5 (Ops and AluCommand with derives) ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
struct AluCommand {
    a: u8,
    b: u8,
    op: Ops,
}

// ---- Figure code (verbatim from the book) ----

use std::fmt;

impl fmt::Display for AluCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cmd: a=0x{:02x} b=0x{:02x} op={:?}", self.a, self.b, self.op)
    }
}

fn main() {
    let cmd = AluCommand { a: 0xAA, b: 0x55, op: Ops::Xor };
    println!("{}", cmd);
}
