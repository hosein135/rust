// Rust for RTL Verification — Chapter 35, Figure 6
// "clone_from is the UVM's copy(): fill an object you already have"
// Run with: cargo run --bin ch35_fig06_clone_from_is_uvm_copy
//
// Expected output:
//   before: Name: , ID: 0 Grades: []
//   after:  Name: Mary, ID: 33 Grades: [97, 82]
//   -- and clone() is clone(): a new one, returned --
//   fresh:  Name: Mary, ID: 33 Grades: [97, 82]

use std::fmt;

#[derive(Debug, Clone, Default)]
struct StudentRecord {
    name: String,
    id_number: u32,
    grades: Vec<u32>,
}

impl fmt::Display for StudentRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {} Grades: {:?}", self.name, self.id_number, self.grades)
    }
}

fn main() {
    let mary = StudentRecord {
        name: String::from("Mary"),
        id_number: 33,
        grades: vec![97, 82],
    };

    // The UVM has two copying calls and Rust has the same two, under the same
    // names:
    //
    //   uvm_object.copy(other)  ->  Clone::clone_from(&mut self, source)
    //   uvm_object.clone()      ->  Clone::clone(&self)
    //
    // `copy()` fills in an object you already have. So does `clone_from`.
    let mut mary_copy = StudentRecord::default();
    println!("before: {mary_copy}");

    mary_copy.clone_from(&mary);
    println!("after:  {mary_copy}");

    // `clone()` hands back a new one.
    println!("-- and clone() is clone(): a new one, returned --");
    let fresh = mary.clone();
    println!("fresh:  {fresh}");

    // What has no counterpart is `do_copy()` itself, and the discipline that
    // comes with it — "always call `super().do_copy(other)` first", which the
    // UVM needs because a copy has to walk up an inheritance chain. Rust has
    // no inheritance, so the derive walks the field list instead and there is
    // no first step to forget.
    //
    // One false friend to note: Rust's `Copy` trait is unrelated to the UVM's
    // `copy()`. `Copy` means "duplicating this is a memcpy, and the original
    // stays usable" — it is about ownership, not about transactions. A
    // transaction that owns a `String` or a `Vec` cannot be `Copy` at all.
}
