// Rust for RTL Verification — Chapter 35, Figure 4
// "Clone is deep for what you own"
// Run with: cargo run --bin ch35_fig04_clone_is_deep_for_what_you_own
//
// Expected output:
//   mary:      Name: Mary, ID: 33 Grades: [97, 82]
//   mary_copy: Name: Mary, ID: 33 Grades: [97, 82]
//   -- add a grade to the copy --
//   mary:      Name: Mary, ID: 33 Grades: [97, 82]
//   mary_copy: Name: Mary, ID: 33 Grades: [97, 82, 100]

use std::fmt;

// A student is a person who also owns a list of grades. The `Vec` is the
// interesting part: it is the field that would make a shallow copy visible.
#[derive(Debug, Clone)]
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

    // `Clone` is `do_copy()`, generated from the field list. Cloning a field
    // you *own* clones its contents — so `mary_copy` gets its own `Vec`, not
    // a second name for Mary's.
    //
    // Python has to offer both `copy.copy()` and `copy.deepcopy()` because
    // assignment shares a reference and the shallow form is the default. In
    // Rust there is one `clone()`, and how deep it goes is already written in
    // the type: owned data is copied, and sharing has to be asked for
    // (Figure 5).
    let mut mary_copy = mary.clone();

    println!("mary:      {mary}");
    println!("mary_copy: {mary_copy}");

    println!("-- add a grade to the copy --");
    mary_copy.grades.push(100);

    // Mary is untouched. There is no way for this to have gone the other way:
    // the compiler would not have let the two share a `Vec` without an
    // explicit `Rc`.
    println!("mary:      {mary}");
    println!("mary_copy: {mary_copy}");
}
