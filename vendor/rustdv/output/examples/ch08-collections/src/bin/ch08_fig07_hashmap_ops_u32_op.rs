// Rust for RTL Verification — Chapter 8, Figure 7
// "A HashMap<Ops, u32> op-frequency counter"
// Run with: cargo run --bin ch08_fig07_hashmap_ops_u32_op
//
// Expected output:
//   Mul: 2
//   Xor: 1
//   Add: 3


use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

fn main() {
    let op_stream = vec![Ops::Add, Ops::Mul, Ops::Add,
                         Ops::Xor, Ops::Add, Ops::Mul];
    let mut freq: HashMap<Ops, u32> = HashMap::new();
    for op in &op_stream {
        *freq.entry(*op).or_insert(0) += 1;
    }
    for (op, count) in &freq {
        println!("{:?}: {}", op, count);
    }
}
