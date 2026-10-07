// Rust for RTL Verification — Chapter 2, Figure 5
// "The compiler as proofreader"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0425]: cannot find value `resutl` in this scope
//    --> src/main.rs:3:20
//     |
//   3 |     println!("{}", resutl);
//     |                    ^^^^^^ help: a local variable with a similar name
//     |                            exists: `result`


fn main() {
    let result = 42;
    println!("{}", resutl);
}
