// Rust for RTL Verification — Chapter 4, Figure 10
// "The compiler catches missing cases"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0004]: non-exhaustive patterns: `0_u8` and `5_u8..=u8::MAX` not covered
//    --> src/main.rs:3:22
//     |
//   3 |     let name = match op_code {
//     |                      ^^^^^^^ not covered
//     |
//     = note: the matched value is of type `u8`


fn main() {
    let op_code: u8 = 2;
    let name = match op_code {
        1 => "ADD",
        2 => "AND",
        3 => "XOR",
        4 => "MUL",
    };
    println!("{name}");
}
