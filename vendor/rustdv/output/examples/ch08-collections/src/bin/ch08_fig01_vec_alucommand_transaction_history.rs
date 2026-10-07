// Rust for RTL Verification — Chapter 8, Figure 1
// "A Vec<AluCommand> as a transaction history log"
// Run with: cargo run --bin ch08_fig01_vec_alucommand_transaction_history
//
// Expected output:
//   2 commands logged
//   first: AluCommand { a: 5, b: 3, op: Add }


#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

#[derive(Clone, Debug, PartialEq)]
struct AluCommand { a: u8, b: u8, op: Ops }

fn main() {
    let mut log: Vec<AluCommand> = Vec::new();
    log.push(AluCommand { a: 5, b: 3, op: Ops::Add });
    log.push(AluCommand { a: 2, b: 2, op: Ops::Mul });

    println!("{} commands logged", log.len());
    println!("first: {:?}", log[0]);
}
