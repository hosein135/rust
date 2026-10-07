// Rust for RTL Verification — Chapter 14, Figure 6
// "Unit tests live beside the code they test"
// Run the tests: cargo test


// src/predictor.rs

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_carries_into_bit_eight() {
        assert_eq!(alu_prediction(0xFF, 0xFF, Ops::Add), 0x01FE);
    }

    #[test]
    fn and_masks_operands() {
        assert_eq!(alu_prediction(0xF0, 0x3C, Ops::And), 0x0030);
    }

    #[test]
    fn xor_finds_differing_bits() {
        assert_eq!(alu_prediction(0xF0, 0x3C, Ops::Xor), 0x00CC);
    }

    #[test]
    fn mul_needs_the_full_result_bus() {
        assert_eq!(alu_prediction(0xFF, 0xFF, Ops::Mul), 0xFE01);
    }
}
