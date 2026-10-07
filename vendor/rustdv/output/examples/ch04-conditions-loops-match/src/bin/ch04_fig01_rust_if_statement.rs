// Rust for RTL Verification — Chapter 4, Figure 1
// "A Rust if statement"
// Run with: cargo run --bin ch04_fig01_rust_if_statement
//
// Expected output:
//   Hey, you're not Danny.


fn main() {
    let name = "Roy";
    if name != "Danny" {
        println!("Hey, you're not Danny.");
    }
}
