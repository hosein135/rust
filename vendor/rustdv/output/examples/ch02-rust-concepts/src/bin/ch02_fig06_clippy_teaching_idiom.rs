// Rust for RTL Verification — Chapter 2, Figure 6
// "Clippy teaching idiom"
// Run with: cargo run --bin ch02_fig06_clippy_teaching_idiom
//
// Expected output:
//   % cargo clippy
//   warning: equality checks against true are unnecessary
//    --> src/main.rs:3:8
//     |
//   3 |     if done == true {
//     |        ^^^^^^^^^^^^ help: try simplifying it as shown: `done`


fn main() {
    let done = true;
    if done == true {
        println!("finished");
    }
}
