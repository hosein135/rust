// Rust for RTL Verification — Chapter 6, Figure 2
// "An exclusive reference grants write access"
// Run with: cargo run --bin ch06_fig02_exclusive_reference_grants_write
//
// Expected output:
//   After scramble: 99


struct Transaction {
    data: u8,
}

fn scramble(t: &mut Transaction) {
    t.data = 99;
}

fn main() {
    let mut t = Transaction { data: 42 };   // mut: the owner permits mutation

    scramble(&mut t);   // lend write access, briefly

    println!("After scramble: {}", t.data);
}
