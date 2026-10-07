// Rust for RTL Verification — Chapter 9, Figure 5
// "The ? operator sends the error up the stack"
// Run with: cargo run --bin ch09_fig05_operator_sends_error_up
//
// Expected output:
//   percent failed: DivideByZero


// ---- Context from Chapter 9, Figure 3 (DivError and nice_div) ----

#[derive(Debug)]
enum DivError {
    DivideByZero,
}

fn nice_div(dividend: u32, divisor: u32) -> Result<u32, DivError> {
    match dividend.checked_div(divisor) {
        Some(result) => Ok(result),
        None => Err(DivError::DivideByZero),
    }
}

// ---- Figure code (verbatim from the book) ----

fn percent(numerator: u32, denominator: u32) -> Result<u32, DivError> {
    let ratio = nice_div(numerator * 100, denominator)?;
    Ok(ratio)
}

fn main() {
    match percent(40, 0) {
        Ok(pct) => println!("{pct}%"),
        Err(err) => println!("percent failed: {err:?}"),
    }
}
