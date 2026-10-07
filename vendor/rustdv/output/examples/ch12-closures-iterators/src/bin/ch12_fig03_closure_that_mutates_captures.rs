// Rust for RTL Verification — Chapter 12, Figure 3
// "A closure that mutates captures by exclusive borrow"
// Run with: cargo run --bin ch12_fig03_closure_that_mutates_captures
//
// Expected output:
//   2 errors: ["ADD result mismatch", "XOR result mismatch"]


fn main() {
    let mut errors = Vec::new();

    let mut log_error = |msg: &str| errors.push(msg.to_string());

    log_error("ADD result mismatch");
    log_error("XOR result mismatch");

    println!("{} errors: {errors:?}", errors.len());
}
