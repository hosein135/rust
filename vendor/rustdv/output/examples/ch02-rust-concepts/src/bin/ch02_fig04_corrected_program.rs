// Rust for RTL Verification — Chapter 2, Figure 4
// "The corrected program"
// Run with: cargo run --bin ch02_fig04_corrected_program
//
// Expected output:
//   256


fn main() {
    let a: u8 = 0xFF;
    let b: u8 = 0x01;
    let result: u16 = a as u16 + b as u16;
    let reg: u16 = result;
    println!("{}", reg);
}
