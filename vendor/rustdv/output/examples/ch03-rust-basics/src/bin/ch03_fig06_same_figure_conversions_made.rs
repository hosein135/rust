// Rust for RTL Verification — Chapter 3, Figure 6
// "The same figure, with the conversions made explicit"
// Run with: cargo run --bin ch03_fig06_same_figure_conversions_made
//
// Expected output:
//   ss: 3
//   dd: 1


fn main() {
    let ii: i32 = 1;
    let ff: f64 = 2.0;
    let ss = ii as f64 + ff;
    println!("ss: {ss}");
    let dd = ii / ii;
    println!("dd: {dd}");
}
