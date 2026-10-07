// Rust for RTL Verification — Chapter 12, Figure 7
// "fold reduces a stream to one value"
// Run with: cargo run --bin ch12_fig07_fold_reduces_stream_one
//
// Expected output:
//   mismatches: 1


fn main() {
    let results = [(0x55u16, 0x55u16), (0x100, 0x100), (0x0FE, 0x0FF)];

    let mismatches = results
        .iter()
        .fold(0, |errs, (exp, act)| if exp == act { errs } else { errs + 1 });

    println!("mismatches: {mismatches}");
}
