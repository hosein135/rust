// Rust for RTL Verification — Chapter 8, Figure 5
// "String literals are borrowed; String is owned"
// Run with: cargo run --bin ch08_fig05_string_literals_are_borrowed
//
// Expected output:
//   TinyALU
//   *** Testing the TinyALU ***


fn main() {
    let dut: &str = "TinyALU";            // borrowed slice into program memory
    let mut name: String = String::from("Tiny");
    name.push_str("ALU");                 // owned and growable
    let banner = format!("*** Testing the {} ***", name);
    println!("{}", dut);
    println!("{}", banner);
}
