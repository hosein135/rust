// Rust for RTL Verification — Chapter 13, Figure 7
// "Chapter 5's Python experiment, finally legal in Rust — with its costs itemized"
// Run with: cargo run --bin ch13_fig07_chapter_5_s_python
//
// Expected output:
//   Transaction { op: "MUL", a: 5, b: 3 }
//   same object: true


use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Transaction {
    op: String,
    a: u8,
    b: u8,
}

fn main() {
    let a = Rc::new(RefCell::new(Transaction {
        op: String::from("ADD"),
        a: 5,
        b: 3,
    }));
    let b = Rc::clone(&a);          // two names...

    b.borrow_mut().op = String::from("MUL");   // ...mutate through one...

    println!("{:?}", a.borrow());              // ...observe through the other
    println!("same object: {}", Rc::ptr_eq(&a, &b));
}
