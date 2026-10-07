// Rust for RTL Verification — Chapter 3, Figure 10
// "if is an expression"
// Run with: cargo run --bin ch03_fig10_if_expression
//
// Expected output:
//   count is even


fn main() {
    let count: u8 = 42;
    let parity = if count % 2 == 0 { "even" } else { "odd" };
    println!("count is {parity}");
}
