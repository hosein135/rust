// Rust for RTL Verification — Chapter 35, Figure 1
// "A transaction is a plain struct with two string forms"
// Run with: cargo run --bin ch35_fig01_two_string_forms
//
// Expected output:
//   Printing the record: Name: Joe Shmoe, ID: 37
//   Logging the record:  Name: Joe Shmoe, ID: 37
//   Debug form:          PersonRecord { name: "Joe Shmoe", id_number: 37 }

use std::fmt;

// No base class. `uvm_object` gave a transaction a name, a printer, a
// comparer and a copier; in Rust each of those is a trait you derive or
// write, so there is nothing left for a base class to hold.
#[derive(Debug)]
struct PersonRecord {
    name: String,
    id_number: u32,
}

// `Display` is `convert2string()` / `__str__()`. It is **not** derivable —
// you write it, because only you know which fields are worth reading.
impl fmt::Display for PersonRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {}", self.name, self.id_number)
    }
}

fn main() {
    let xx = PersonRecord { name: String::from("Joe Shmoe"), id_number: 37 };

    // `{}` asks for Display: the form you chose.
    println!("Printing the record: {xx}");
    println!("Logging the record:  {}", xx.to_string());

    // `{:?}` asks for Debug: every field, mechanically, for free. Useful when
    // you are debugging the testbench rather than reading the transaction.
    println!("Debug form:          {xx:?}");
}
