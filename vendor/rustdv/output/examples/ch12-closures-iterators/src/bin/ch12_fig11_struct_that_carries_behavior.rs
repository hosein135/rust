// Rust for RTL Verification — Chapter 12, Figure 11
// "A struct that carries its behavior as a closure"
// Run with: cargo run --bin ch12_fig11_struct_that_carries_behavior
//
// Expected output:
//   PASS: (255, 1) -> 256
//   FAIL: (15, 53) expected 5, got 6


struct Checker {
    predict: Box<dyn Fn(u8, u8) -> u16>,
}

impl Checker {
    fn check(&self, aa: u8, bb: u8, actual: u16) {
        let expected = (self.predict)(aa, bb);
        if expected == actual {
            println!("PASS: ({aa}, {bb}) -> {actual}");
        } else {
            println!("FAIL: ({aa}, {bb}) expected {expected}, got {actual}");
        }
    }
}

fn main() {
    let adder_check = Checker {
        predict: Box::new(|aa, bb| aa as u16 + bb as u16),
    };
    adder_check.check(0xFF, 0x01, 0x100);

    let and_check = Checker {
        predict: Box::new(|aa, bb| (aa & bb) as u16),
    };
    and_check.check(0x0F, 0x35, 0x0006);  // wrong on purpose
}
