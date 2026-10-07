// Rust for RTL Verification — Chapter 6, Figure 4
// "The same actors, with the ordering made real"
// Run with: cargo run --bin ch06_fig04_same_actors_ordering_made
//
// Expected output:
//   Scoreboard checking 42


fn main() {
    let mut transaction_data: Option<u8> = None;

    let monitor = &mut transaction_data;
    *monitor = Some(42);                  // the monitor writes...
                                          // ...and its borrow ends at its last use

    let scoreboard = &transaction_data;   // now the reader may claim access
    match scoreboard {
        Some(data) => println!("Scoreboard checking {data}"),
        None => println!("Nothing to check yet"),
    }
}
