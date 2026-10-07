// Rust for RTL Verification — Chapter 3, Figure 11
// "A block is an expression; the semicolon is the switch"
// Run with: cargo run --bin ch03_fig11_block_expression_semicolon_switch
//
// Expected output:
//   nn: 7


fn main() {
    let nn = {
        let doubled = 2 * 3;
        doubled + 1
    };
    println!("nn: {nn}");
}
