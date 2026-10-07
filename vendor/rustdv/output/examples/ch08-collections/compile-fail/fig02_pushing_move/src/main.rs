// Rust for RTL Verification — Chapter 8, Figure 2
// "Pushing is a move"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0382]: borrow of moved value: `cmd`
//     --> src/main.rs:12:28
//      |
//   10 |     let cmd = AluCommand { a: 5, b: 3, op: Ops::Add };
//      |         --- move occurs because `cmd` has type `AluCommand`,
//      |             which does not implement the `Copy` trait
//   11 |     log.push(cmd);
//      |              --- value moved here
//   12 |     println!("sent: {:?}", cmd);
//      |                            ^^^ value borrowed here after move
//      |
//   help: consider cloning the value if the performance cost is acceptable
//      |
//   11 |     log.push(cmd.clone());
//      |                 ++++++++


// ---- Context from Chapter 8, Figure 1 (definitions this figure needs) ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

#[derive(Clone, Debug, PartialEq)]
struct AluCommand { a: u8, b: u8, op: Ops }

// ---- Figure code (verbatim from the book) ----

fn main() {
    let mut log: Vec<AluCommand> = Vec::new();
    let cmd = AluCommand { a: 5, b: 3, op: Ops::Add };
    log.push(cmd);
    println!("sent: {:?}", cmd);   // cmd moved into the Vec
}
