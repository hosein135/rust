// Rust for RTL Verification — Chapter 12, Figure 5
// "The list comprehension, as an iterator chain"
// Run with: cargo run --bin ch12_fig05_list_comprehension_iterator_chain
//
// Expected output:
//   even squares [0, 4, 16, 36, 64, 100]


fn main() {
    let even_squares: Vec<u32> = (0..=10)
        .filter(|nn| nn % 2 == 0)
        .map(|nn| nn * nn)
        .collect();

    println!("even squares {even_squares:?}");
}
