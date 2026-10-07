// Rust for RTL Verification — Chapter 2, Figure 3
// "The mistake SystemVerilog would have allowed"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0308]: mismatched types
//    --> src/main.rs:5:19
//     |
//   5 |     let reg: u8 = result;
//     |              --   ^^^^^^ expected `u8`, found `u16`
//     |              |
//     |              expected due to this
//     |
//   help: you can convert a `u16` to a `u8` and panic if the converted value
//         doesn't fit
//     |
//   5 |     let reg: u8 = result.try_into().unwrap();
//     |                         ++++++++++++++++++++
//   For more information about this error, try `rustc --explain E0308`.


fn main() {
    let a: u8 = 0xFF;
    let b: u8 = 0x01;
    let result: u16 = a as u16 + b as u16;
    let reg: u8 = result;
    println!("{}", reg);
}
