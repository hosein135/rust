// Rust for RTL Verification — Chapter 7, Figure 1
// "Defining and instantiating a struct"
// Run with: cargo run --bin ch07_fig01_defining_instantiating_struct
//
// Expected output:
//   Walrus mass: 1000


struct Animal {
    kg: f64,
}

fn main() {
    let walrus = Animal { kg: 1000.0 };
    println!("Walrus mass: {}", walrus.kg);
}
