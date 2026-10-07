// Rust for RTL Verification — Chapter 10, Figure 2
// "SmallDog by composition — delegation replaces super()"
// Run with: cargo run --bin ch10_fig02_smalldog_composition_delegation_replaces
//
// Expected output:
//   The dog says 'yap yap'


// ---- Context from Chapter 10, Figure 1 (the Animal trait and Dog) ----

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

// ---- Figure code (verbatim from the book) ----

struct SmallDog {
    dog: Dog,    // a SmallDog HAS the dog parts
}

impl Animal for SmallDog {
    fn species(&self) -> &str { self.dog.species() }   // delegate to Dog
    fn sound(&self) -> &str { "yap yap" }              // override
}

fn main() {
    let sd = SmallDog { dog: Dog };
    sd.make_sound();
}
