// Rust for RTL Verification — Chapter 5, Figure 4
// "Copy types don't move — small values are simply copied"
// Run with: cargo run --bin ch05_fig04_copy_types_don_t
//
// Expected output:
//   a = 5, b = 5


fn main() {
    let a: u8 = 5;
    let b = a;              // copies the byte; a is still alive
    println!("a = {a}, b = {b}");
}
