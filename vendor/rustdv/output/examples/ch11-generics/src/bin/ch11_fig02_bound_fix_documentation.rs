// Rust for RTL Verification — Chapter 11, Figure 2
// "The bound is the fix — and the documentation"
// Run with: cargo run --bin ch11_fig02_bound_fix_documentation
//
// Expected output:
//   largest operand: 0xaa
//   largest result:  0x7100
//   last test name:  alu_xor_test


fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    let operands: Vec<u8> = vec![0x22, 0xAA, 0x07];
    let results: Vec<u16> = vec![0x0154, 0x7100, 0x00FF];
    let tests = vec!["alu_add_test", "alu_xor_test", "alu_mul_test"];

    println!("largest operand: 0x{:02x}", largest(&operands));
    println!("largest result:  0x{:04x}", largest(&results));
    println!("last test name:  {}", largest(&tests));
}
