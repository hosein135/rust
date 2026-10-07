// Rust for RTL Verification — Chapter 9, Figure 1
// "A HashMap lookup returns Option"
// Run with: cargo run --bin ch09_fig01_hashmap_lookup_returns_option
//
// Expected output:
//   Number 4? Not in database
//   Number 4? Not in database


use std::collections::HashMap;

fn main() {
    let mut players = HashMap::new();
    players.insert(7, "Beckham");
    players.insert(10, "Messi");
    players.insert(11, "Salah");

    match players.get(&4) {
        Some(name) => println!("Number 4? {name}"),
        None => println!("Number 4? Not in database"),
    }

    let player = players.get(&4).copied().unwrap_or("Not in database");
    println!("Number 4? {player}");
}
