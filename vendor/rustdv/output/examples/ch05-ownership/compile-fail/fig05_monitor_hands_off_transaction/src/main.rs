// Rust for RTL Verification — Chapter 5, Figure 5
// "The monitor hands off a transaction — and learns what "hands off" means"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0382]: borrow of moved value: `t`
//     --> src/main.rs:15:44
//      |
//   13 |     let t = Transaction { a: 5, b: 3, op: 1 };
//      |         - move occurs because `t` has type `Transaction`, which
//      |           does not implement the `Copy` trait
//   14 |     scoreboard(t);
//      |                - value moved here
//   15 |     println!("monitor logging: a was {}", t.a);
//      |                                           ^^^ value borrowed here after move
//      |
//   note: consider changing this parameter type in function `scoreboard` to
//         borrow instead if owning the value isn't necessary
//   help: consider cloning the value if the performance cost is acceptable


struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn scoreboard(t: Transaction) {
    println!("scoreboard checking: {} op {} (code {})", t.a, t.b, t.op);
}   // <- t dropped here: the scoreboard owned it, the scoreboard's
    //    scope ends, the transaction is destroyed. Question answered.

fn main() {
    // main is playing the monitor today.
    let t = Transaction { a: 5, b: 3, op: 1 };
    scoreboard(t);
    println!("monitor logging: a was {}", t.a);
}
