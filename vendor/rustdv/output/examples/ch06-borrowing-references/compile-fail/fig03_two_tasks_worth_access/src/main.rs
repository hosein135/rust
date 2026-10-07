// Rust for RTL Verification — Chapter 6, Figure 3
// "Two tasks' worth of access to one value -- the borrow checker objects"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0502]: cannot borrow `transaction_data` as immutable because it is also borrowed as mutable
//    --> src/main.rs:5:22
//     |
//   4 |     let monitor = &mut transaction_data;
//     |                   ---------------------- mutable borrow occurs here
//   5 |     let scoreboard = &transaction_data;
//     |                      ^^^^^^^^^^^^^^^^^ immutable borrow occurs here
//   ...
//   7 |     *monitor = Some(42);
//     |     ------------------- mutable borrow later used here
//
//   For more information about this error, try `rustc --explain E0502`.
//   error: could not compile `borrow_race` (bin "borrow_race") due to 1 previous error


fn main() {
    let mut transaction_data: Option<u8> = None;

    let monitor = &mut transaction_data;   // the monitor's claim: write access
    let scoreboard = &transaction_data;    // the scoreboard's claim: read access

    *monitor = Some(42);                   // the monitor sees a transaction...

    match scoreboard {                     // ...and the scoreboard checks it
        Some(data) => println!("Scoreboard checking {data}"),
        None => println!("Nothing to check yet"),
    }
}
