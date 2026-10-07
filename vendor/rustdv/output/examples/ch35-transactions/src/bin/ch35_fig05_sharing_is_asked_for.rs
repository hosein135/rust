// Rust for RTL Verification — Chapter 35, Figure 5
// "If you want the shallow copy, you ask for it"
// Run with: cargo run --bin ch35_fig05_sharing_is_asked_for
//
// Expected output:
//   before: Mary [97, 82] / Mary [97, 82]
//   -- add a grade through the copy --
//   after:  Mary [97, 82, 100] / Mary [97, 82, 100]
//   handles to the one list: 2

use std::cell::RefCell;
use std::rc::Rc;

// Same record, one field changed: the grades now live behind an `Rc`, so a
// clone copies the *handle* and both records point at one list. This is
// Python's `copy.copy()` — except that in Python it is what you get by
// default and here you had to write `Rc` to ask for it.
//
// `RefCell` comes along because sharing something you intend to change means
// the borrow check moves to run time (Chapter 13).
#[derive(Debug, Clone)]
struct SharedStudent {
    name: String,
    grades: Rc<RefCell<Vec<u32>>>,
}

fn main() {
    let mary = SharedStudent {
        name: String::from("Mary"),
        grades: Rc::new(RefCell::new(vec![97, 82])),
    };

    let mary_copy = mary.clone();

    println!(
        "before: {} {:?} / {} {:?}",
        mary.name,
        mary.grades.borrow(),
        mary_copy.name,
        mary_copy.grades.borrow()
    );

    println!("-- add a grade through the copy --");
    mary_copy.grades.borrow_mut().push(100);

    // Both see it, because there is only one list.
    println!(
        "after:  {} {:?} / {} {:?}",
        mary.name,
        mary.grades.borrow(),
        mary_copy.name,
        mary_copy.grades.borrow()
    );

    // The count is the proof: two handles, one list.
    println!("handles to the one list: {}", Rc::strong_count(&mary.grades));
}
