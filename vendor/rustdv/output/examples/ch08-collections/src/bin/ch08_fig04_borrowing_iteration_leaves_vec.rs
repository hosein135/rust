// Rust for RTL Verification — Chapter 8, Figure 4
// "Borrowing iteration leaves the Vec intact"
// Run with: cargo run --bin ch08_fig04_borrowing_iteration_leaves_vec
//
// Expected output:
//   AluCommand { a: 5, b: 3, op: Add }
//   AluCommand { a: 2, b: 2, op: Mul }
//   2 commands still logged


// ---- Context from Chapter 8, Figure 1 (definitions this figure needs) ----

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

#[derive(Clone, Debug, PartialEq)]
struct AluCommand { a: u8, b: u8, op: Ops }

// ---- Figure code (verbatim from the book) ----

fn main() {
    let log = vec![
        AluCommand { a: 5, b: 3, op: Ops::Add },
        AluCommand { a: 2, b: 2, op: Ops::Mul },
    ];
    for cmd in &log {
        println!("{:?}", cmd);
    }
    println!("{} commands still logged", log.len());
}
