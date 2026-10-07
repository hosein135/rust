// Rust for RTL Verification — Chapter 4, Figure 3
// "else if as a switch (for now)"
// Run with: cargo run --bin ch04_fig03_else_if_switch_now
//
// Expected output:
//   Illegal Operation: divide


fn main() {
    let (a, b) = (5, 5);
    let operation = "divide";
    if operation == "add" {
        println!("A + B = {}", a + b);
    } else if operation == "subtract" {
        println!("A - B = {}", a - b);
    } else if operation == "multiply" {
        println!("A * B = {}", a * b);
    } else {
        println!("Illegal Operation: {operation}");
    }
}
