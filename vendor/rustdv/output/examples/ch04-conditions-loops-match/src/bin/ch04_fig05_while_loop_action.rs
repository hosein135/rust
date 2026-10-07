// Rust for RTL Verification — Chapter 4, Figure 5
// "A while loop in action"
// Run with: cargo run --bin ch04_fig05_while_loop_action
//
// Expected output:
//   0 1 2 3 4 5 6 7 8 9 10 11 12 13


fn main() {
    let mut nn = 0;
    while nn <= 13 {
        print!("{nn} ");
        nn += 1;
    }
    println!();
}
