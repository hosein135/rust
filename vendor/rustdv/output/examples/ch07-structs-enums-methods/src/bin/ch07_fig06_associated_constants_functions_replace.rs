// Rust for RTL Verification — Chapter 7, Figure 6
// "Associated constants and functions replace class variables and static methods"
// Run with: cargo run --bin ch07_fig06_associated_constants_functions_replace
//
// Expected output:
//   I have 3 sides.


struct Triangle;

impl Triangle {
    const SIDE_COUNT: u32 = 3;

    fn print_side_count() {
        println!("I have {} sides.", Self::SIDE_COUNT);
    }
}

fn main() {
    Triangle::print_side_count();
}
