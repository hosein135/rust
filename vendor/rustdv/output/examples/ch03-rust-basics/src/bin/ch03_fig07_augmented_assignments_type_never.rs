// Rust for RTL Verification — Chapter 3, Figure 7
// "Augmented assignments — the type never changes"
// Run with: cargo run --bin ch03_fig07_augmented_assignments_type_never
//
// Expected output:
//   xx: 1
//   xx += 1: 2
//   xx *= 3: 6
//   xx /= 4: 1


fn main() {
    let mut xx = 1;
    println!("xx: {xx}");
    xx += 1;
    println!("xx += 1: {xx}");
    xx *= 3;
    println!("xx *= 3: {xx}");
    xx /= 4;
    println!("xx /= 4: {xx}");
}
