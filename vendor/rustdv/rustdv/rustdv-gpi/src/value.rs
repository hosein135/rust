//! 4-state value types: port of cocotb's `Logic`/`LogicArray`
//! (cocotb: types/, design-doc mapping row 23). Conversion failures are
//! `Result`s, not exceptions.

use std::fmt;

use num_bigint::BigUint;

use crate::ValueError;

/// One 4-state bit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Logic {
    Zero,
    One,
    X,
    Z,
}

impl Logic {
    pub fn from_char(c: char) -> Logic {
        match c {
            '0' => Logic::Zero,
            '1' => Logic::One,
            'z' | 'Z' => Logic::Z,
            _ => Logic::X,
        }
    }
    pub fn to_char(self) -> char {
        match self {
            Logic::Zero => '0',
            Logic::One => '1',
            Logic::X => 'x',
            Logic::Z => 'z',
        }
    }
    pub fn is_resolvable(self) -> bool {
        matches!(self, Logic::Zero | Logic::One)
    }
}

impl From<bool> for Logic {
    fn from(b: bool) -> Logic {
        if b { Logic::One } else { Logic::Zero }
    }
}

impl fmt::Display for Logic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// A fixed-width vector of 4-state bits using VPI's `a_val`/`b_val`
/// representation. Bit pairs encode `00=0`, `10=1`, `11=X`, and `01=Z`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicArray {
    a_val: BigUint,
    b_val: BigUint,
    width: usize,
}

impl LogicArray {
    /// Construct from VPI vector words in least-significant-word-first order.
    /// Each tuple is `(aval, bval)`. `width` preserves the exact signal width
    /// when the final word uses fewer than 32 bits.
    pub fn from_vpi_words(words: &[(u32, u32)], width: usize) -> LogicArray {
        let word_count = width.div_ceil(32);
        assert_eq!(
            words.len(),
            word_count,
            "{width}-bit LogicArray needs {word_count} VPI word(s), got {}",
            words.len()
        );

        let mut a_words: Vec<u32> = words.iter().map(|(aval, _)| *aval).collect();
        let mut b_words: Vec<u32> = words.iter().map(|(_, bval)| *bval).collect();
        if !width.is_multiple_of(32) {
            let mask = (1u32 << (width % 32)) - 1;
            *a_words.last_mut().expect("partial width has a VPI word") &= mask;
            *b_words.last_mut().expect("partial width has a VPI word") &= mask;
        }

        LogicArray {
            a_val: BigUint::from_slice(&a_words),
            b_val: BigUint::from_slice(&b_words),
            width,
        }
    }

    pub fn from_binstr(s: &str) -> LogicArray {
        let mut a_val = BigUint::ZERO;
        let mut b_val = BigUint::ZERO;
        let mut width = 0;
        for (bit, c) in s.chars().rev().enumerate() {
            let bit = bit as u64;
            match Logic::from_char(c) {
                Logic::Zero => {}
                Logic::One => a_val.set_bit(bit, true),
                Logic::X => {
                    a_val.set_bit(bit, true);
                    b_val.set_bit(bit, true);
                }
                Logic::Z => b_val.set_bit(bit, true),
            }
            width += 1;
        }
        LogicArray {
            a_val,
            b_val,
            width,
        }
    }

    pub fn from_bool(v: bool, width: usize) -> LogicArray {
        LogicArray::from_u128(v as u128, width)
    }

    pub fn from_u8(v: u8, width: usize) -> LogicArray {
        LogicArray::from_u128(v as u128, width)
    }

    pub fn from_u16(v: u16, width: usize) -> LogicArray {
        LogicArray::from_u128(v as u128, width)
    }

    pub fn from_u32(v: u32, width: usize) -> LogicArray {
        LogicArray::from_u128(v as u128, width)
    }

    pub fn from_u64(v: u64, width: usize) -> LogicArray {
        LogicArray::from_u128(v as u128, width)
    }

    pub fn from_u128(v: u128, width: usize) -> LogicArray {
        let mut a_val = BigUint::ZERO;
        for bit in 0..width.min(u128::BITS as usize) {
            if v & (1u128 << bit) != 0 {
                a_val.set_bit(bit as u64, true);
            }
        }
        LogicArray {
            a_val,
            b_val: BigUint::ZERO,
            width,
        }
    }

    pub fn from_bigint(v: &BigUint, width: usize) -> LogicArray {
        let mask = (BigUint::from(1u8) << width) - BigUint::from(1u8);
        LogicArray {
            a_val: v & mask,
            b_val: BigUint::ZERO,
            width,
        }
    }

    /// Return VPI vector words in least-significant-word-first order.
    /// Each tuple is `(aval, bval)`.
    pub fn to_vpi_words(&self) -> Vec<(u32, u32)> {
        let word_count = self.width.div_ceil(32);
        let mut a_words = self.a_val.iter_u32_digits();
        let mut b_words = self.b_val.iter_u32_digits();
        (0..word_count)
            .map(|_| (a_words.next().unwrap_or(0), b_words.next().unwrap_or(0)))
            .collect()
    }

    pub fn to_binstr(&self) -> String {
        (0..self.width)
            .rev()
            .map(|index| {
                self[index]
                    .expect("index generated from LogicArray width")
                    .to_char()
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.width
    }
    pub fn is_empty(&self) -> bool {
        self.width == 0
    }

    /// Return true if the value is resolvable to a 2-state value (no X/Z bits).
    pub fn is_resolvable(&self) -> bool {
        self.b_val == BigUint::ZERO
    }
}

impl TryFrom<&LogicArray> for bool {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<bool, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val != BigUint::ZERO)
    }
}

impl TryFrom<&LogicArray> for u8 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u8, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val.iter_u32_digits().next().unwrap_or(0) as u8)
    }
}

impl TryFrom<&LogicArray> for u16 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u16, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val.iter_u32_digits().next().unwrap_or(0) as u16)
    }
}

impl TryFrom<&LogicArray> for u32 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u32, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val.iter_u32_digits().next().unwrap_or(0))
    }
}

impl TryFrom<&LogicArray> for u64 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u64, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val.iter_u64_digits().next().unwrap_or(0))
    }
}

impl TryFrom<&LogicArray> for u128 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u128, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        let mut digits = v.a_val.iter_u64_digits();
        let low = digits.next().unwrap_or(0) as u128;
        let high = digits.next().unwrap_or(0) as u128;
        Ok(low | (high << 64))
    }
}

impl TryFrom<&LogicArray> for BigUint {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<BigUint, ValueError> {
        if !v.is_resolvable() {
            return Err(ValueError::FourState(v.to_binstr()));
        }
        Ok(v.a_val.clone())
    }
}

/// Access bits least-significant-bit first, returning `None` out of bounds.
impl std::ops::Index<usize> for LogicArray {
    type Output = Option<Logic>;

    fn index(&self, index: usize) -> &Self::Output {
        // Return references to the decoded logic states.
        static ZERO: Option<Logic> = Some(Logic::Zero);
        static ONE: Option<Logic> = Some(Logic::One);
        static X: Option<Logic> = Some(Logic::X);
        static Z: Option<Logic> = Some(Logic::Z);
        // Check if the index is out of bounds first
        if index >= self.width {
            return &None;
        }
        // Return a reference to the appropriate static value based on
        // a_val and b_val
        match (self.a_val.bit(index as u64), self.b_val.bit(index as u64)) {
            (false, false) => &ZERO,
            (true, false) => &ONE,
            (true, true) => &X,
            (false, true) => &Z,
        }
    }
}

impl fmt::Display for LogicArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_binstr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u64() {
        let v = LogicArray::from_u64(0xA5, 8);
        assert_eq!(v.to_binstr(), "10100101");
        assert_eq!(u64::try_from(&v).unwrap(), 0xA5);
    }

    #[test]
    fn constructs_from_two_state_primitive_types() {
        let v = LogicArray::from_bool(true, 1);
        assert_eq!(v.to_binstr(), "1");

        let v = LogicArray::from_u8(0xa5, 8);
        assert_eq!(u8::try_from(&v), Ok(0xa5));

        let v = LogicArray::from_u16(0xa5b6, 16);
        assert_eq!(u16::try_from(&v), Ok(0xa5b6));

        let v = LogicArray::from_u32(0xa5b6_c7d8, 32);
        assert_eq!(u32::try_from(&v), Ok(0xa5b6_c7d8));

        let v = LogicArray::from_u64(0x0123_4567_89ab_cdef, 64);
        assert_eq!(u64::try_from(&v), Ok(0x0123_4567_89ab_cdef));

        let value = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210;
        let v = LogicArray::from_u128(value, 128);
        assert_eq!(u128::try_from(&v), Ok(value));

        let truncated = LogicArray::from_u8(0xa5, 4);
        assert_eq!(truncated.to_binstr(), "0101");
    }

    #[test]
    fn bigint_conversion_preserves_arbitrary_width_values() {
        let value = BigUint::from_slice(&[
            0x7654_3210,
            0xfedc_ba98,
            0x89ab_cdef,
            0x0123_4567,
            0xa5a5_5a5a,
        ]);
        let v = LogicArray::from_bigint(&value, 160);

        assert_eq!(v.len(), 160);
        assert_eq!(BigUint::try_from(&v), Ok(value));

        let truncated = LogicArray::from_bigint(&BigUint::from(0xffu8), 4);
        assert_eq!(BigUint::try_from(&truncated), Ok(BigUint::from(0x0fu8)));
    }

    #[test]
    fn converts_to_two_state_primitive_types() {
        let zero = LogicArray::from_binstr("0");
        assert_eq!(bool::try_from(&zero), Ok(false));

        let v = LogicArray::from_vpi_words(
            &[
                (0x7654_3210, 0),
                (0xfedc_ba98, 0),
                (0x89ab_cdef, 0),
                (0x0123_4567, 0),
            ],
            128,
        );
        assert_eq!(bool::try_from(&v), Ok(true));
        assert_eq!(u8::try_from(&v), Ok(0x10));
        assert_eq!(u16::try_from(&v), Ok(0x3210));
        assert_eq!(u32::try_from(&v), Ok(0x7654_3210));
        assert_eq!(u64::try_from(&v), Ok(0xfedc_ba98_7654_3210));
        assert_eq!(
            u128::try_from(&v),
            Ok(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        );
    }

    #[test]
    fn four_state_rejected() {
        let v = LogicArray::from_binstr("1x0z");
        assert!(bool::try_from(&v).is_err());
        assert!(u8::try_from(&v).is_err());
        assert!(u16::try_from(&v).is_err());
        assert!(u32::try_from(&v).is_err());
        assert!(u64::try_from(&v).is_err());
        assert!(u128::try_from(&v).is_err());
        assert!(BigUint::try_from(&v).is_err());
        assert!(!v.is_resolvable());
    }

    #[test]
    fn vpi_encoding_and_lsb_first_bit_access() {
        let v = LogicArray::from_binstr("01xz");

        assert_eq!(v.a_val, BigUint::from(0b0110u8));
        assert_eq!(v.b_val, BigUint::from(0b0011u8));
        assert_eq!(v[0], Some(Logic::Z));
        assert_eq!(v[1], Some(Logic::X));
        assert_eq!(v[2], Some(Logic::One));
        assert_eq!(v[3], Some(Logic::Zero));
        assert_eq!(v[4], None);
        assert_eq!(v.to_binstr(), "01xz");
    }

    #[test]
    fn constructs_from_vpi_words_and_masks_unused_bits() {
        let states = LogicArray::from_vpi_words(&[(0b0110, 0b0011)], 4);
        assert_eq!(states.to_binstr(), "01xz");
        assert_eq!(states.to_vpi_words(), vec![(0b0110, 0b0011)]);

        let partial = LogicArray::from_vpi_words(&[(0xffff_fff5, 0xffff_fff8)], 3);
        assert_eq!(partial.to_binstr(), "101");
        assert!(partial.is_resolvable());
        assert_eq!(partial.to_vpi_words(), vec![(0b101, 0)]);
    }

    #[test]
    fn width_is_preserved_independently_of_biguint_magnitude() {
        let zeros = LogicArray::from_binstr("00000");
        assert_eq!(zeros.len(), 5);
        assert_eq!(zeros.to_binstr(), "00000");

        let wide = LogicArray::from_u64(1, 65);
        assert_eq!(wide.len(), 65);
        assert_eq!(wide[0], Some(Logic::One));
        assert_eq!(wide[64], Some(Logic::Zero));
        assert_eq!(u64::try_from(&wide), Ok(1));

        let empty = LogicArray::from_binstr("");
        assert!(empty.is_empty());
        assert_eq!(empty.to_binstr(), "");
    }
}
