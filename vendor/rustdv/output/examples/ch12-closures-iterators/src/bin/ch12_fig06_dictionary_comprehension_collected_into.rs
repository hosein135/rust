// Rust for RTL Verification — Chapter 12, Figure 6
// "The dictionary comprehension, collected into a HashMap"
// Run with: cargo run --bin ch12_fig06_dictionary_comprehension_collected_into
//
// Expected output:
//   cubes: {2: 8, 0: 0, 3: 27, 1: 1}


use std::collections::HashMap;

fn main() {
    let cubes: HashMap<u32, u32> = (0..4)
        .map(|ii: u32| (ii, ii.pow(3)))
        .collect();

    println!("cubes: {cubes:?}");
}
