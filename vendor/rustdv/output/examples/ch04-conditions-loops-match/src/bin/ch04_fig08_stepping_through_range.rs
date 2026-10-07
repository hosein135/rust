// Rust for RTL Verification — Chapter 4, Figure 8
// "Stepping through a range"
// Run with: cargo run --bin ch04_fig08_stepping_through_range
//
// Expected output:
//   1 3 5 7 9 11 13


fn main() {
    for ii in (1..14).step_by(2) {
        print!("{ii} ");
    }
    println!();
}
