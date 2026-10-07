// Rust for RTL Verification — Chapter 14, Figure 1
// "A module declared inline, in the middle of main.rs"
// Run with: cargo run --bin ch14_fig01_module_declared_inline_middle
//
// Expected output:
//   0xFF + 0x01 = 0x0100


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

fn main() {
    let sum = predictor::alu_prediction(0xFF, 0x01, predictor::Ops::Add);
    println!("0xFF + 0x01 = {sum:#06x}");
}
