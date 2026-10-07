// Rust for RTL Verification — Chapter 10, Figure 3
// "Multiple roles as multiple trait implementations"
// Run with: cargo run --bin ch10_fig03_multiple_roles_multiple_trait
//
// Expected output:
//   Pat gives the baby a kiss.
//   Pat sprays water.


trait Parent {
    fn kiss(&self);
}

trait Firefighter {
    fn hose(&self);
}

struct Pat {
    name: String,
}

impl Parent for Pat {
    fn kiss(&self) { println!("{} gives the baby a kiss.", self.name); }
}

impl Firefighter for Pat {
    fn hose(&self) { println!("{} sprays water.", self.name); }
}

fn main() {
    let pat = Pat { name: String::from("Pat") };
    pat.kiss();
    pat.hose();
}
