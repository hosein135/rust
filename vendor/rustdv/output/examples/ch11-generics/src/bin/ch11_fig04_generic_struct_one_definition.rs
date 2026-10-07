// Rust for RTL Verification — Chapter 11, Figure 4
// "A generic struct — one definition, many widths"
// Run with: cargo run --bin ch11_fig04_generic_struct_one_definition
//
// Expected output:
//   A: 0xaa  result: 0x7100


struct Register<T> {
    value: T,
}

impl<T: Copy> Register<T> {
    fn new(value: T) -> Self {
        Register { value }
    }
    fn read(&self) -> T {
        self.value
    }
    fn write(&mut self, value: T) {
        self.value = value;
    }
}

fn main() {
    let mut a_reg: Register<u8> = Register::new(0x00);   // the A leg is 8 bits
    let mut result: Register<u16> = Register::new(0x0000); // the result is 16

    a_reg.write(0xAA);
    result.write(0x7100);
    println!("A: 0x{:02x}  result: 0x{:04x}", a_reg.read(), result.read());
}
