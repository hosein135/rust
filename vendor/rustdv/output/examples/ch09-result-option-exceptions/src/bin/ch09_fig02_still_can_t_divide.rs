// Rust for RTL Verification — Chapter 9, Figure 2
// "You still can't divide by zero"
// Run with: cargo run --bin ch09_fig02_still_can_t_divide
// NOTE: this figure PANICS ON PURPOSE — a nonzero exit is the lesson.
//
// Expected output:
//   thread 'main' panicked at src/main.rs:3:26:
//   attempt to divide by zero
//   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


fn main() {
    let divisor: i32 = "0".parse().unwrap();
    println!("3/0 = {}", 3 / divisor);
}
