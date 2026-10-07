// Rust for RTL Verification — Chapter 9, Figure 3
// "nice_div returns a Result instead of raising"
// Run with: cargo run --bin ch09_fig03_nice_div_returns_result
//
// Expected output:
//   nice_div(33, 2) = 16
//   You screwed up your division, human: DivideByZero


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

fn main() {
    match nice_div(33, 2) {
        Ok(result) => println!("nice_div(33, 2) = {result}"),
        Err(err) => println!("You screwed up your division, human: {err:?}"),
    }
    match nice_div(3, 0) {
        Ok(result) => println!("nice_div(3, 0) = {result}"),
        Err(err) => println!("You screwed up your division, human: {err:?}"),
    }
}
