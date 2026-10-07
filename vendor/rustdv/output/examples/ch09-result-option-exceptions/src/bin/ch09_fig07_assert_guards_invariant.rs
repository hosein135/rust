// Rust for RTL Verification — Chapter 9, Figure 7
// "assert! guards an invariant"
// Run with: cargo run --bin ch09_fig07_assert_guards_invariant
// NOTE: this figure PANICS ON PURPOSE — a nonzero exit is the lesson.
//
// Expected output:
//   checksum: 0x11
//   thread 'main' panicked at src/main.rs:16:5:
//   checksum self-test failed: 0x11 ^ 0x12 != 0
//   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


fn xor_bytes(bytes: &[u8]) -> u8 {
    let mut xor = 0;
    for b in bytes {
        xor ^= b;
    }
    xor
}

fn main() {
    let frame = [0x08, 0x09, 0x10];
    let checksum = xor_bytes(&frame);
    println!("checksum: {checksum:#04x}");
    assert!(
        xor_bytes(&[checksum, 0x11]) == 0,
        "checksum self-test failed: {checksum:#04x} ^ 0x11 != 0"
    );
    assert!(
        xor_bytes(&[checksum, 0x12]) == 0,
        "checksum self-test failed: {checksum:#04x} ^ 0x12 != 0"
    );
    println!("all self-tests passed");
}
