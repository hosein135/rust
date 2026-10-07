// Rust for RTL Verification — Chapter 7, Figure 7
// "The Ops enumeration and an exhaustive match"
// Run with: cargo run --bin ch07_fig07_ops_enumeration_exhaustive_match
//
// Expected output:
//   Add of 0x0A and 0x05 -> 0x000f


#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
    match op {
        Ops::Add => a as u16 + b as u16,
        Ops::And => (a & b) as u16,
        Ops::Xor => (a ^ b) as u16,
        Ops::Mul => a as u16 * b as u16,
    }
}

fn main() {
    let op = Ops::Add;
    println!("{:?} of 0x0A and 0x05 -> 0x{:04x}", op, alu_prediction(0x0A, 0x05, op));
}
