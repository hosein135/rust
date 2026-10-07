// Rust for RTL Verification — Chapter 8, Figure 6
// "Take &str; accept everything"
// Run with: cargo run --bin ch08_fig06_take_str_accept_everything
//
// Expected output:
//   PASSED: smoke_test
//   PASSED: random_ops
//   still have random_ops


fn report_pass(test_name: &str) {
    println!("PASSED: {}", test_name);
}

fn main() {
    let owned = String::from("random_ops");
    report_pass("smoke_test");    // a literal is already a &str
    report_pass(&owned);          // a &String coerces to &str
    println!("still have {}", owned);
}
