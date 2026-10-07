// Rust for RTL Verification — Chapter 3, Figure 12
// "A TinyALU prediction function"
// Run with: cargo run --bin ch03_fig12_tinyalu_prediction_function
//
// Expected output:
//   predicted sum: 0x01fe


fn predict_add(aa: u8, bb: u8) -> u16 {
    aa as u16 + bb as u16
}

fn main() {
    let sum = predict_add(0xFF, 0xFF);
    println!("predicted sum: {sum:#06x}");
}
