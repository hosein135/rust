// Rust for RTL Verification — Chapter 4, Figure 9
// "match as a switch"
// Run with: cargo run --bin ch04_fig09_match_switch
//
// Expected output:
//   answer = 25


fn main() {
    let (a, b) = (5, 5);
    let operation = "multiply";
    let answer = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        _ => panic!("Illegal Operation: {operation}"),
    };
    println!("answer = {answer}");
}
