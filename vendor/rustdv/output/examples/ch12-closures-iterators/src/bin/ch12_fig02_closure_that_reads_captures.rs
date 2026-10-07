// Rust for RTL Verification — Chapter 12, Figure 2
// "A closure that reads captures by shared borrow"
// Run with: cargo run --bin ch12_fig02_closure_that_reads_captures
//
// Expected output:
//   ops under test: ["ADD", "AND", "XOR", "MUL"]
//   ops under test: ["ADD", "AND", "XOR", "MUL"]
//   still mine: 4 ops


fn main() {
    let ops = vec!["ADD", "AND", "XOR", "MUL"];

    let show = || println!("ops under test: {ops:?}");

    show();
    show();
    println!("still mine: {} ops", ops.len());  // ops was only borrowed
}
