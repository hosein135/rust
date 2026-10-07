// Rust for RTL Verification — Chapter 11, Figure 3
// "Multiple bounds with a where clause"
// Run with: cargo run --bin ch11_fig03_multiple_bounds_where_clause
//
// Expected output:
//   MISMATCH: expected 84, actual 85


use std::fmt::Debug;

fn check_match<T>(expected: &T, actual: &T) -> bool
where
    T: PartialEq + Debug,
{
    if expected == actual {
        true
    } else {
        println!("MISMATCH: expected {:?}, actual {:?}", expected, actual);
        false
    }
}

fn main() {
    check_match(&0x54u16, &0x54u16);
    check_match(&0x54u16, &0x55u16);
}
