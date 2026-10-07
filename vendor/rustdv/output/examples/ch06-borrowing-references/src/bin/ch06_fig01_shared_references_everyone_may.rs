// Rust for RTL Verification — Chapter 6, Figure 1
// "Shared references — everyone may look, nobody may touch"
// Run with: cargo run --bin ch06_fig01_shared_references_everyone_may
//
// Expected output:
//   Saw transaction with data 42
//   Saw transaction with data 42
//   Still the owner: 42


struct Transaction {
    data: u8,
}

fn report(t: &Transaction) {
    println!("Saw transaction with data {}", t.data);
}

fn main() {
    let t = Transaction { data: 42 };

    let monitor_view = &t;    // a borrow
    let coverage_view = &t;   // another borrow -- readers may alias freely

    report(monitor_view);
    report(coverage_view);

    println!("Still the owner: {}", t.data);   // t was never moved
}
