// Rust for RTL Verification — Chapter 4, Figure 7
// "Looping through numbers using a range"
// Run with: cargo run --bin ch04_fig07_looping_through_numbers_using
//
// Expected output:
//   0 1 2 3


fn main() {
    for ii in 0..4 {
        print!("{ii} ");
    }
    println!();
}
