// Rust for RTL Verification — Chapter 9, Figure 4
// "The TypeError scenario, ported to Rust"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0308]: mismatched types
//    --> src/main.rs:4:24
//     |
//   4 |     match nice_div(3, "zero") {
//     |           -------- ^^^^^^ expected `u32`, found `&str`
//     |           |
//     |           arguments to this function are incorrect


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

fn main() {
    match nice_div(3, "zero") {
        Ok(result) => println!("nice_div = {result}"),
        Err(err) => println!("Error: {err:?}"),
    }
}
