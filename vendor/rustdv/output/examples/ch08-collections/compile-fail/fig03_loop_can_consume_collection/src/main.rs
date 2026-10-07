// Rust for RTL Verification — Chapter 8, Figure 3
// "A for loop can consume the collection"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0382]: borrow of moved value: `log`
//     --> src/main.rs:14:29
//      |
//   9  |     for cmd in log {
//      |                --- `log` moved due to this implicit call
//      |                    to `.into_iter()`
//   ...
//   14 |     println!("{} commands", log.len());
//      |                             ^^^ value borrowed here after move
//      |
//   help: consider borrowing to avoid moving into the for loop
//      |
//   9  |     for cmd in &log {
//      |                +


// ---- Context from Chapter 8, Figure 1 (definitions this figure needs) ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

#[derive(Clone, Debug, PartialEq)]
struct AluCommand { a: u8, b: u8, op: Ops }

// ---- Figure code (verbatim from the book) ----

fn main() {
    let log = vec![
        AluCommand { a: 5, b: 3, op: Ops::Add },
        AluCommand { a: 2, b: 2, op: Ops::Mul },
    ];
    for cmd in log {
        println!("{:?}", cmd);
    }
    println!("{} commands", log.len());   // log is gone
}
