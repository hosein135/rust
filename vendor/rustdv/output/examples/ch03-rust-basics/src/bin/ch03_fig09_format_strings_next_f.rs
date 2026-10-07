// Rust for RTL Verification — Chapter 3, Figure 9
// "Format strings, next to the f-strings you know"
// Run with: cargo run --bin ch03_fig09_format_strings_next_f
//
// Expected output:
//   aa is 42 and bb is 7
//   aa is 42 and bb is 7
//   aa in hex: 0x2a
//   aa in binary: 0b00101010
//   sum: 49


fn main() {
    let aa: u8 = 0x2A;
    let bb: u8 = 7;
    println!("aa is {} and bb is {}", aa, bb);  // like str.format()
    println!("aa is {aa} and bb is {bb}");      // like an f-string
    println!("aa in hex: {aa:#04x}");
    println!("aa in binary: {aa:#010b}");
    println!("sum: {}", aa + bb);
}
