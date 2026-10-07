// Rust for RTL Verification — Chapter 7, Figure 4
// "A method that mutates takes &mut self"
// Run with: cargo run --bin ch07_fig04_method_that_mutates_takes
//
// Expected output:
//   Walrus mass after lunch: 1003.5


struct Animal {
    kg: f64,
}

impl Animal {
    fn feed(&mut self, meal_kg: f64) {
        self.kg += meal_kg;
    }
}

fn main() {
    let mut walrus = Animal { kg: 1000.0 };
    walrus.feed(3.5);
    println!("Walrus mass after lunch: {}", walrus.kg);
}
