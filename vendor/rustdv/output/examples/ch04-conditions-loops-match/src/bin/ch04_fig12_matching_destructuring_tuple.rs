// Rust for RTL Verification — Chapter 4, Figure 12
// "Matching and destructuring a tuple"
// Run with: cargo run --bin ch04_fig12_matching_destructuring_tuple
//
// Expected output:
//   one operand zero


fn main() {
    let operands: (u8, u8) = (0, 200);
    let comment = match operands {
        (0, 0) => String::from("both operands zero"),
        (0, _) | (_, 0) => String::from("one operand zero"),
        (a, b) if a == b => format!("equal operands: {a}"),
        (a, b) => format!("ordinary operands: {a}, {b}"),
    };
    println!("{comment}");
}
