// Rust for RTL Verification — Chapter 3, Figure 3
// "A mutable binding"
// Run with: cargo run --bin ch03_fig03_mutable_binding
//
// Expected output:
//   xx: 5
//   xx: 6


fn main() {
    let mut xx = 5;
    println!("xx: {xx}");
    xx = 6;
    println!("xx: {xx}");
}
