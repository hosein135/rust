// Rust for RTL Verification — Chapter 13, Figure 1
// "A Box owns its value on the heap — everything else is Chapter 5"
// Run with: cargo run --bin ch13_fig01_box_owns_value_heap
//
// Expected output:
//   boxed transaction: 5 op 3


struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn main() {
    let t = Box::new(Transaction { a: 5, b: 3, op: 1 });
    println!("boxed transaction: {} op {}", t.a, t.b);
}   // t goes out of scope; the Box is dropped; the heap memory is freed.
    // One owner, one drop, on time — nothing new.
