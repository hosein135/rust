// Rust for RTL Verification — Chapter 5, Figure 6
// "The monitor keeps a copy — explicitly"
// Run with: cargo run --bin ch05_fig06_monitor_keeps_copy_explicitly
//
// Expected output:
//   scoreboard checking: 5 op 3 (code 1)
//   monitor logging: a was 5


#[derive(Clone)]
struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn scoreboard(t: Transaction) {
    println!("scoreboard checking: {} op {} (code {})", t.a, t.b, t.op);
}

fn main() {
    let t = Transaction { a: 5, b: 3, op: 1 };
    scoreboard(t.clone());   // the scoreboard owns the copy...
    println!("monitor logging: a was {}", t.a);   // ...the monitor owns the original
}
