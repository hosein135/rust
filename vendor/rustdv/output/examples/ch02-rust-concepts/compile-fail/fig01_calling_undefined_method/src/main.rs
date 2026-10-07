// Rust for RTL Verification — Chapter 2, Figure 1
// "Calling an undefined method"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   % cargo run
//      Compiling concepts v0.1.0
//   error[E0599]: no method named `ends_with` found for type `u8` in the current scope
//    --> src/main.rs:5:26
//     |
//   5 |     println!("{}", myint.ends_with("a whimper"));
//     |                          ^^^^^^^^^ method not found in `u8`
//
//   error: could not compile `concepts` (bin "concepts") due to 1 previous error


fn main() {
    let mystring = "Hello, World";
    println!("{}", mystring.ends_with("orld"));
    let myint: u8 = 42;
    println!("{}", myint.ends_with("a whimper"));
}
