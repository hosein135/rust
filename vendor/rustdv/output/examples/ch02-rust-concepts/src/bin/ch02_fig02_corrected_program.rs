// Rust for RTL Verification — Chapter 2, Figure 2
// "The corrected program"
// Run with: cargo run --bin ch02_fig02_corrected_program
//
// Expected output:
//   true


fn main() {
    let mystring = "Hello, World";
    println!("{}", mystring.ends_with("orld"));
}
