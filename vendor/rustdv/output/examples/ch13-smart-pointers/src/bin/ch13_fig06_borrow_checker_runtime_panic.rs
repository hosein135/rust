// Rust for RTL Verification — Chapter 13, Figure 6
// "The borrow checker at runtime — a panic replaces the compile error"
// Run with: cargo run --bin ch13_fig06_borrow_checker_runtime_panic
// NOTE: this figure PANICS ON PURPOSE — a nonzero exit is the lesson.
//
// Expected output:
//   thread 'main' panicked at src/main.rs:12:25:
//   already borrowed: BorrowMutError
//   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


use std::cell::RefCell;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = RefCell::new(Scoreboard { errors: 0 });

    let reader = sb.borrow();          // a reader is at the whiteboard...
    let mut writer = sb.borrow_mut();  // ...and a writer grabs the marker

    writer.errors += 1;
    println!("reader saw: {}", reader.errors);
}
