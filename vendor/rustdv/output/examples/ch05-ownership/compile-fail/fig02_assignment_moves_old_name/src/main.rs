// Rust for RTL Verification — Chapter 5, Figure 2
// "Assignment moves — and the old name is gone"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0382]: borrow of moved value: `a`
//    --> src/main.rs:4:15
//     |
//   2 |     let a = String::from("ADD 5 3");
//     |         - move occurs because `a` has type `String`, which does not
//     |           implement the `Copy` trait
//   3 |     let b = a;
//     |             - value moved here
//   4 |     println!("{a}");
//     |               ^^^ value borrowed here after move
//     |
//   help: consider cloning the value if the performance cost is acceptable
//     |
//   3 |     let b = a.clone();
//     |              ++++++++


fn main() {
    let a = String::from("ADD 5 3");
    let b = a;
    println!("{a}");
    println!("{b}");
}
