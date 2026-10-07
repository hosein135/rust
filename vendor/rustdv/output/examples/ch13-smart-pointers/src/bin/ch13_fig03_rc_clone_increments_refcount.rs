// Rust for RTL Verification — Chapter 13, Figure 3
// "Rc::clone increments a refcount — on purpose, where you can see it"
// Run with: cargo run --bin ch13_fig03_rc_clone_increments_refcount
//
// Expected output:
//   owners: 1
//   owners: 3
//   monitor sees: TinyALU BFM
//   owners: 2
//   driver sees: TinyALU BFM


use std::rc::Rc;

fn main() {
    let bfm = Rc::new(String::from("TinyALU BFM"));
    println!("owners: {}", Rc::strong_count(&bfm));

    let driver_handle = Rc::clone(&bfm);
    {
        let monitor_handle = Rc::clone(&bfm);
        println!("owners: {}", Rc::strong_count(&bfm));
        println!("monitor sees: {monitor_handle}");
    }   // monitor_handle dropped here: count decrements

    println!("owners: {}", Rc::strong_count(&bfm));
    println!("driver sees: {driver_handle}");
}
