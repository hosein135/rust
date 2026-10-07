// Rust for RTL Verification — Chapter 4, Figure 4
// "Conditional assignment — no ternary needed"
// Run with: cargo run --bin ch04_fig04_conditional_assignment_no_ternary
//
// Expected output:
//   five_val


fn main() {
    let aa = 5;
    let message = if aa == 5 { "five_val" } else { "other_val" };
    println!("{message}");
}
