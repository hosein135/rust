// Chapter 21, Figure 2: A declarative macro — patterns in, code out
// Run: cargo run --bin ch21_fig02_a_declarative_macro

macro_rules! check {
    ($actual:expr, $expected:expr) => {
        if $actual == $expected {
            println!("PASSED: {} = {:04x}", stringify!($actual), $actual);
        } else {
            println!(
                "FAILED: {} = {:04x} - predicted {:04x}",
                stringify!($actual),
                $actual,
                $expected
            );
        }
    };
}

fn main() {
    let result: u16 = 0xFF + 0x01;
    check!(result, 0x0100);
    check!(result, 0x0000);
}
