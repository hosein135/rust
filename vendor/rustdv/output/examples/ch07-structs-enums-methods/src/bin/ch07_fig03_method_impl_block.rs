// Rust for RTL Verification — Chapter 7, Figure 3
// "A method in an impl block"
// Run with: cargo run --bin ch07_fig03_method_impl_block
//
// Expected output:
//   Walrus weight in pounds 454.55


struct Animal {
    kg: f64,
}

impl Animal {
    fn get_pounds(&self) -> f64 {
        self.kg / 2.2
    }
}

fn main() {
    let walrus = Animal { kg: 1000.0 };
    println!("Walrus weight in pounds {:.2}", walrus.get_pounds());
}
