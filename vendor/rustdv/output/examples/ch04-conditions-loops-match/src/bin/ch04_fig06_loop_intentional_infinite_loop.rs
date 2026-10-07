// Rust for RTL Verification — Chapter 4, Figure 6
// "loop — the intentional infinite loop"
// Run with: cargo run --bin ch04_fig06_loop_intentional_infinite_loop
//
// Expected output:
//   225


fn main() {
    let mut nn = 0;
    let first_big_square = loop {
        nn += 1;
        if nn * nn > 200 {
            break nn * nn;
        }
    };
    println!("{first_big_square}");
}
