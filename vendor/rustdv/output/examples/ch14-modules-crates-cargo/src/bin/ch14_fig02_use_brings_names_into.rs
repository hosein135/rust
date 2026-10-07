// Rust for RTL Verification — Chapter 14, Figure 2
// "use brings names into scope, like Python's from-import"
// Run with: cargo run --bin ch14_fig02_use_brings_names_into
//
// Expected output:
//   AND: 0x0030
//   XOR: 0x00cc


mod predictor {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

    pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
        match op {
            Ops::Add => a as u16 + b as u16,
            Ops::And => (a & b) as u16,
            Ops::Xor => (a ^ b) as u16,
            Ops::Mul => a as u16 * b as u16,
        }
    }
}

use predictor::{alu_prediction, Ops};

fn main() {
    println!("AND: {:#06x}", alu_prediction(0xF0, 0x3C, Ops::And));
    println!("XOR: {:#06x}", alu_prediction(0xF0, 0x3C, Ops::Xor));
}
