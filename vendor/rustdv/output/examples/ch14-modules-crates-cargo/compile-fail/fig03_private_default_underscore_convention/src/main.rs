// Rust for RTL Verification — Chapter 14, Figure 3
// "Private by default — the underscore convention, enforced"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0603]: function `widen` is private
//     --> src/main.rs:20:24
//      |
//   20 |     let w = predictor::widen(0xFF);  // reaching for a private helper
//      |                        ^^^^^ private function
//      |
//   note: the function `widen` is defined here
//     --> src/main.rs:5:5
//      |
//   5  |     fn widen(x: u8) -> u16 {     // no pub: private to this module
//      |     ^^^^^^^^^^^^^^^^^^^^^^


mod predictor {
    pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

    fn widen(x: u8) -> u16 {     // no pub: private to this module
        x as u16
    }

    pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
        match op {
            Ops::Add => widen(a) + widen(b),
            Ops::And => widen(a & b),
            Ops::Xor => widen(a ^ b),
            Ops::Mul => widen(a) * widen(b),
        }
    }
}

fn main() {
    let w = predictor::widen(0xFF);  // reaching for a private helper
    println!("{w}");
}
