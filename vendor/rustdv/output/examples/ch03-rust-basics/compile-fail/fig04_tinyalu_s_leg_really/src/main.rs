// Rust for RTL Verification — Chapter 3, Figure 4
// "The TinyALU's A leg really is a u8"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   % cargo run
//   error: literal out of range for `u8`
//    --> src/main.rs:3:18
//     |
//   3 |     let bb: u8 = 300;
//     |                  ^^^
//     |
//     = note: the literal `300` does not fit into the type `u8`
//       whose range is `0..=255`


fn main() {
    let aa: u8 = 0xFF;
    let bb: u8 = 300;
    println!("aa: {aa}, bb: {bb}");
}
