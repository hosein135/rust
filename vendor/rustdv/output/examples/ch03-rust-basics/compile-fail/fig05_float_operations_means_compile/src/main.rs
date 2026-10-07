// Rust for RTL Verification — Chapter 3, Figure 5
// "A float in operations means... a compile error"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   % cargo run
//   error[E0277]: cannot add a `f64` to `i32`
//    --> src/main.rs:4:17
//     |
//   4 |     let ss = ii + ff;
//     |                 ^ no implementation for `i32 + f64`


fn main() {
    let ii: i32 = 1;
    let ff: f64 = 2.0;
    let ss = ii + ff;
    println!("ss: {ss}");
}
