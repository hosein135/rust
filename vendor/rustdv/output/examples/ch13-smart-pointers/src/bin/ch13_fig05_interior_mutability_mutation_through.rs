// Rust for RTL Verification — Chapter 13, Figure 5
// "Interior mutability — mutation through an immutable binding"
// Run with: cargo run --bin ch13_fig05_interior_mutability_mutation_through
//
// Expected output:
//   errors: 1


use std::cell::RefCell;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = RefCell::new(Scoreboard { errors: 0 });   // note: no `mut`

    sb.borrow_mut().errors += 1;    // the write handle lives for this line only

    println!("errors: {}", sb.borrow().errors);
}
