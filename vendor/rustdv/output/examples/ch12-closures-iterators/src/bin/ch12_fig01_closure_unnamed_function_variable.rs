// Rust for RTL Verification — Chapter 12, Figure 1
// "A closure is an unnamed function in a variable"
// Run with: cargo run --bin ch12_fig01_closure_unnamed_function_variable
//
// Expected output:
//   42
//   255 + 1 = 256


fn main() {
    let add_one = |x: u32| x + 1;

    let describe = |aa: u8, bb: u8| {
        let sum = aa as u16 + bb as u16;
        format!("{aa} + {bb} = {sum}")
    };

    println!("{}", add_one(41));
    println!("{}", describe(0xFF, 0x01));
}
