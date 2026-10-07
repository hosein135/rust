// Rust for RTL Verification — Chapter 7, Figure 10
// "An enum whose variants carry payloads"
// Run with: cargo run --bin ch07_fig10_enum_whose_variants_carry
//
// Expected output:
//   FAIL: expected 0x000f, got 0x0005


#[derive(Debug)]
enum CheckResult {
    Pass,
    Fail { expected: u16, actual: u16 },
}

fn check(expected: u16, actual: u16) -> CheckResult {
    if expected == actual {
        CheckResult::Pass
    } else {
        CheckResult::Fail { expected, actual }
    }
}

fn main() {
    let result = check(0x000f, 0x0005);
    match result {
        CheckResult::Pass => println!("PASS"),
        CheckResult::Fail { expected, actual } => {
            println!("FAIL: expected 0x{expected:04x}, got 0x{actual:04x}")
        }
    }
}
