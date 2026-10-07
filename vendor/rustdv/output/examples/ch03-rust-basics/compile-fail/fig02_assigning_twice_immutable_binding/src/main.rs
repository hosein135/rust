// Rust for RTL Verification — Chapter 3, Figure 2
// "Assigning twice to an immutable binding"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   % cargo run
//   error[E0384]: cannot assign twice to immutable variable `xx`
//    --> src/main.rs:4:5
//     |
//   2 |     let xx = 5;
//     |         -- first assignment to `xx`
//   3 |     println!("xx: {xx}");
//   4 |     xx = 6;
//     |     ^^^^^^ cannot assign twice to immutable variable
//     |
//   help: consider making this binding mutable
//     |
//   2 |     let mut xx = 5;
//     |         +++


fn main() {
    let xx = 5;
    println!("xx: {xx}");
    xx = 6;
    println!("xx: {xx}");
}
