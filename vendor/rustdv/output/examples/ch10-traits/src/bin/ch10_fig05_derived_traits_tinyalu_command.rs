// Rust for RTL Verification — Chapter 10, Figure 5
// "Derived traits on the TinyALU command transaction"
// Run with: cargo run --bin ch10_fig05_derived_traits_tinyalu_command
//
// Expected output:
//   equal? true
//   AluCommand { a: 170, b: 85, op: Xor }


#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
struct AluCommand {
    a: u8,
    b: u8,
    op: Ops,
}

fn main() {
    let cmd = AluCommand { a: 0xAA, b: 0x55, op: Ops::Xor };
    let copy = cmd.clone();
    println!("equal? {}", cmd == copy);
    println!("{:?}", cmd);
}
