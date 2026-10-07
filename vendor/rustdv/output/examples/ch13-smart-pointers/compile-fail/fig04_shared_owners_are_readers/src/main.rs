// Rust for RTL Verification — Chapter 13, Figure 4
// "Shared owners are readers — Rc will not hand out write access"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0594]: cannot assign to data in an `Rc`
//    --> src/main.rs:9:5
//     |
//   9 |     handle.errors += 1;
//     |     ^^^^^^^^^^^^^^^^^^ cannot assign
//     |
//     = help: trait `DerefMut` is required to modify through a dereference,
//             but it is not implemented for `Rc<Scoreboard>`


use std::rc::Rc;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = Rc::new(Scoreboard { errors: 0 });
    let handle = Rc::clone(&sb);
    handle.errors += 1;
    println!("errors: {}", sb.errors);
}
