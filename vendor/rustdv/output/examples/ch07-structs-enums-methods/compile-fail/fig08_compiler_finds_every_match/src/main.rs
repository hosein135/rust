// Rust for RTL Verification — Chapter 7, Figure 8
// "The compiler finds every match the new variant breaks"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0004]: non-exhaustive patterns: `Ops::Sub` not covered
//     --> src/main.rs:11:11
//      |
//   11 |     match op {
//      |           ^^ pattern `Ops::Sub` not covered
//      |
//   note: `Ops` defined here
//      = note: the matched value is of type `Ops`
//   help: ensure that all possible cases are being handled by
//         adding a match arm with an explicit pattern


#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
    Sub = 5,   // the new operation — and the only edit we made
}

// ---- Context from Chapter 7, Figure 7 (the match the new variant breaks) ----

fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
    match op {
        Ops::Add => a as u16 + b as u16,
        Ops::And => (a & b) as u16,
        Ops::Xor => (a ^ b) as u16,
        Ops::Mul => a as u16 * b as u16,
    }
}

fn main() {
    let op = Ops::Add;
    println!("{:?} of 0x0A and 0x05 -> 0x{:04x}", op, alu_prediction(0x0A, 0x05, op));
}
