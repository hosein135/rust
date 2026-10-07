// Rust for RTL Verification — Chapter 12, Figure 9
// "A TinyALU operand-pair stream, replacing a generator function"
// Run with: cargo run --bin ch12_fig09_tinyalu_operand_pair_stream
//
// Expected output:
//   (0,0) (0,1) (0,2) (1,0) (1,1) (1,2) (2,0) (2,1) (2,2)


fn operand_pairs(n: u8) -> impl Iterator<Item = (u8, u8)> {
    let mut pairs = Vec::new();
    for aa in 0..n {
        for bb in 0..n {
            pairs.push((aa, bb));
        }
    }
    pairs.into_iter()
}

fn main() {
    for (aa, bb) in operand_pairs(3) {
        print!("({aa},{bb}) ");
    }
    println!();
}
