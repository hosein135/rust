// Rust for RTL Verification — Chapter 9, Figure 6
// "A custom error enum for the TinyALU"
// Run with: cargo run --bin ch09_fig06_custom_error_enum_tinyalu
//
// Expected output:
//   1 decodes to Add
//   4 decodes to Mul
//   decode failed: invalid ALU op code: 0x07


// ---- Context from Chapter 8, Figure 1 (the Ops enumeration) ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

// ---- Figure code (verbatim from the book) ----

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum AluError {
    InvalidOp(u8),
    Timeout,
}

impl fmt::Display for AluError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AluError::InvalidOp(bits) => {
                write!(f, "invalid ALU op code: {bits:#04x}")
            }
            AluError::Timeout => write!(f, "timed out waiting for the DUT"),
        }
    }
}

fn decode_op(bits: u8) -> Result<Ops, AluError> {
    match bits {
        1 => Ok(Ops::Add),
        2 => Ok(Ops::And),
        3 => Ok(Ops::Xor),
        4 => Ok(Ops::Mul),
        _ => Err(AluError::InvalidOp(bits)),
    }
}

fn main() {
    for bits in [1, 4, 7] {
        match decode_op(bits) {
            Ok(op) => println!("{bits} decodes to {op:?}"),
            Err(err) => println!("decode failed: {err}"),
        }
    }
}
