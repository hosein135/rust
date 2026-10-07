// Rust for RTL Verification — Chapter 10, Figure 7
// "Generic function — dispatch resolved at compile time"
// Run with: cargo run --bin ch10_fig07_generic_function_dispatch_resolved
//
// Expected output:
//   The dog says 'bow bow'
//   The cat says 'miāo'


// ---- Context from Chapter 10, Figure 1 (Animal, Dog, and Cat) ----

trait Animal {
    fn species(&self) -> &str;
    fn sound(&self) -> &str;

    fn make_sound(&self) {
        println!("The {} says '{}'", self.species(), self.sound());
    }
}

struct Dog;

impl Animal for Dog {
    fn species(&self) -> &str { "dog" }
    fn sound(&self) -> &str { "bow bow" }
}

struct Cat;

impl Animal for Cat {
    fn species(&self) -> &str { "cat" }
    fn sound(&self) -> &str { "miāo" }
}

// ---- Figure code (verbatim from the book) ----

fn check_in<T: Animal>(animal: &T) {
    animal.make_sound();
}

fn main() {
    check_in(&Dog);
    check_in(&Cat);
}
