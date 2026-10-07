// Rust for RTL Verification — Chapter 3, Figure 8
// "Creating a number from a string, by shadowing"
// Run with: cargo run --bin ch03_fig08_creating_number_from_string
//
// Expected output:
//   pi: 3.14159


fn main() {
    let pi = "3.14159";
    let pi: f64 = pi.parse().expect("not a number");
    println!("pi: {pi}");
}
