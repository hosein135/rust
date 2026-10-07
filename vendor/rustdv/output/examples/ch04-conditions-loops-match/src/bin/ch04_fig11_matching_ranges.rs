// Rust for RTL Verification — Chapter 4, Figure 11
// "Matching on ranges"
// Run with: cargo run --bin ch04_fig11_matching_ranges
//
// Expected output:
//   This operation takes 3 cycle(s)


fn main() {
    let op_code: u8 = 4;
    let cycles = match op_code {
        1..=3 => 1,
        4 => 3,
        _ => panic!("Illegal op code: {op_code}"),
    };
    println!("This operation takes {cycles} cycle(s)");
}
