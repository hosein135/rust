// Rust for RTL Verification — Chapter 35, Figure 2
// "You decide what 'the same' means"
// Run with: cargo run --bin ch35_fig02_you_decide_what_same_means
//
// Expected output:
//   -- derived: every field must match --
//   batman == bruce_wayne? false
//   -- written by hand: only the ID counts --
//   batman == bruce_wayne? true
//   Batman is really Bruce Wayne!

use std::fmt;

// `#[derive(PartialEq)]` compares **every field**. That is the undemanding
// default, and it is usually what a transaction wants.
#[derive(Debug, PartialEq)]
struct StrictRecord {
    name: String,
    id_number: u32,
}

// But "the same" is a decision, not a fact. The UVM makes you write
// `do_compare()` for exactly this reason. Here two records are the same
// person when the ID matches, whatever the name says.
#[derive(Debug)]
struct PersonRecord {
    name: String,
    id_number: u32,
}

impl PartialEq for PersonRecord {
    fn eq(&self, other: &PersonRecord) -> bool {
        self.id_number == other.id_number
    }
}

impl fmt::Display for PersonRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {}", self.name, self.id_number)
    }
}

fn main() {
    println!("-- derived: every field must match --");
    let a = StrictRecord { name: String::from("Batman"), id_number: 27 };
    let b = StrictRecord { name: String::from("Bruce Wayne"), id_number: 27 };
    println!("batman == bruce_wayne? {}", a == b);

    println!("-- written by hand: only the ID counts --");
    let batman = PersonRecord { name: String::from("Batman"), id_number: 27 };
    let bruce_wayne = PersonRecord { name: String::from("Bruce Wayne"), id_number: 27 };
    println!("batman == bruce_wayne? {}", batman == bruce_wayne);

    if batman == bruce_wayne {
        println!("Batman is really Bruce Wayne!");
    } else {
        println!("Who is Batman?");
    }
}
