// Rust for RTL Verification — Chapter 7, Figure 5
// "The new() associated function"
// Run with: cargo run --bin ch07_fig05_new_associated_function
//
// Expected output:
//   The Yorkie weighs 9.1 pounds


struct Animal {
    kg: f64,
}

impl Animal {
    fn new(kg: f64) -> Self {
        Self { kg }
    }

    fn get_pounds(&self) -> f64 {
        self.kg / 2.2
    }
}

fn main() {
    let yorkie = Animal::new(20.0);
    println!("The Yorkie weighs {:.1} pounds", yorkie.get_pounds());
}
