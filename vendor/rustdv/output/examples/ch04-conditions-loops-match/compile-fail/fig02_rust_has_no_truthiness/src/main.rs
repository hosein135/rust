// Rust for RTL Verification — Chapter 4, Figure 2
// "Rust has no truthiness"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0308]: mismatched types
//    --> src/main.rs:3:8
//     |
//   3 |     if nn {
//     |        ^^ expected `bool`, found integer


fn main() {
    let nn = 5;
    if nn {
        println!("nonzero");
    }
}
