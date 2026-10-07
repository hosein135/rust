//! Transactions: plain structs with std derives (design-doc §5.1/§7.2,
//! review-memo R1). `Clone` is do_copy, `PartialEq` is do_compare,
//! `Debug` is convert2string — no rustdv base trait.

/// Port of `Ops(IntEnum)` from the book's tinyalu_utils.
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

    pub fn as_u64(self) -> u64 {
        self as u64
    }

    pub fn from_u64(v: u64) -> Option<Ops> {
        match v {
            1 => Some(Ops::Add),
            2 => Some(Ops::And),
            3 => Some(Ops::Xor),
            4 => Some(Ops::Mul),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}

/// Golden model (the scoreboard's predictor). Mirrors tinyalu.sv:
/// single_cycle zero-extends to 16 bits; mult is a plain 8x8→16 multiply.
pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}

// Pure-Rust unit tests, no simulator (design-doc §7.3, convention 5 —
// "a genuinely new capability vs. the Python stack").
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_add_carries_into_bit8() {
        let r = predict(&AluCommand {
            a: 0xFF,
            b: 0xFF,
            op: Ops::Add,
        });
        assert_eq!(r.result, 0x01FE);
    }

    #[test]
    fn predict_all_ops() {
        assert_eq!(
            predict(&AluCommand {
                a: 2,
                b: 3,
                op: Ops::Add
            })
            .result,
            5
        );
        assert_eq!(
            predict(&AluCommand {
                a: 0xF0,
                b: 0x3C,
                op: Ops::And
            })
            .result,
            0x0030
        );
        assert_eq!(
            predict(&AluCommand {
                a: 0xF0,
                b: 0x3C,
                op: Ops::Xor
            })
            .result,
            0x00CC
        );
        assert_eq!(
            predict(&AluCommand {
                a: 4,
                b: 5,
                op: Ops::Mul
            })
            .result,
            20
        );
    }

    #[test]
    fn transaction_derives_do_the_uvm_object_jobs() {
        let t = AluCommand {
            a: 1,
            b: 2,
            op: Ops::Xor,
        };
        let copy = t.clone(); // do_copy
        assert_eq!(t, copy); // do_compare
        let s = format!("{t:?}"); // convert2string
        assert!(s.contains("Xor"));
    }

    #[test]
    fn ops_roundtrip() {
        for op in Ops::ALL {
            assert_eq!(Ops::from_u64(op.as_u64()), Some(op));
        }
        assert_eq!(Ops::from_u64(0), None);
    }
}
