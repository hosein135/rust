// Rust for RTL Verification — Chapter 10, Figure 1
// "Shared behavior through a trait, not a base class"
// Run with: cargo run --bin ch10_fig01_shared_behavior_through_trait
//
// Expected output:
//   The dog says 'bow bow'
//   The cat says 'miāo'


trait Animal {
    fn species(&self) -> &str;
    fn sound(&self) -> &str;

    fn make_sound(&self) {
        println!("The {} says '{}'", self.species(), self.sound());
    }
}

struct Dog;
struct Cat;

impl Animal for Dog {
    fn species(&self) -> &str { "dog" }
    fn sound(&self) -> &str { "bow bow" }
}

impl Animal for Cat {
    fn species(&self) -> &str { "cat" }
    fn sound(&self) -> &str { "miāo" }
}

fn main() {
    Dog.make_sound();
    Cat.make_sound();
}
