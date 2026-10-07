// Rust for RTL Verification — Chapter 7, Figure 9
// "A four-state Logic enum"
// Run with: cargo run --bin ch07_fig09_four_state_logic_enum
//
// Expected output:
//   The signal reads: x


#[derive(Clone, Copy, Debug, PartialEq)]
enum Logic {
    Zero,
    One,
    X,
    Z,
}

fn to_char(v: Logic) -> char {
    match v {
        Logic::Zero => '0',
        Logic::One => '1',
        Logic::X => 'x',
        Logic::Z => 'z',
    }
}

fn main() {
    let bit = Logic::X;
    println!("The signal reads: {}", to_char(bit));
}
