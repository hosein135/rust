// Rust for RTL Verification — Chapter 12, Figure 10
// "The operand-pair stream, built lazily from adapters"
// Run with: cargo run --bin ch12_fig10_operand_pair_stream_lazy
//
// Expected output:
//   (0,0) (0,1) (0,2) (1,0) (1,1) (1,2) (2,0) (2,1) (2,2)


fn operand_pairs(n: u8) -> impl Iterator<Item = (u8, u8)> {
    (0..n).flat_map(move |aa| (0..n).map(move |bb| (aa, bb)))
}

fn main() {
    for (aa, bb) in operand_pairs(3) {
        print!("({aa},{bb}) ");
    }
    println!();
}
