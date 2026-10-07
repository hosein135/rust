// Rust for RTL Verification — Chapter 7, Figure 2
// "Forgetting a field is now a compile error"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0063]: missing field `kg` in initializer of `Animal`
//    --> src/main.rs:6:18
//     |
//   6 |     let yorkie = Animal {};
//     |                  ^^^^^^ missing `kg`


struct Animal {
    kg: f64,
}

fn main() {
    let yorkie = Animal {};
    println!("Yorkie mass: {}", yorkie.kg);
}
