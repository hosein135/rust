// Rust for RTL Verification — Chapter 10, Figure 8
// "Trait objects — dispatch resolved at runtime"
// Run with: cargo run --bin ch10_fig08_trait_objects_dispatch_resolved
//
// Expected output:
//   The dog says 'bow bow'
//   The cat says 'miāo'
//   The dog says 'yap yap'


// ---- Context from Chapter 10, Figures 1 and 2 (Animal, Dog, Cat, SmallDog) ----

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

struct SmallDog {
    dog: Dog,    // a SmallDog HAS the dog parts
}

impl Animal for SmallDog {
    fn species(&self) -> &str { self.dog.species() }   // delegate to Dog
    fn sound(&self) -> &str { "yap yap" }              // override
}

// ---- Figure code (verbatim from the book) ----

fn main() {
    let kennel: Vec<Box<dyn Animal>> = vec![
        Box::new(Dog),
        Box::new(Cat),
        Box::new(SmallDog { dog: Dog }),
    ];
    for animal in &kennel {
        animal.make_sound();
    }
}
