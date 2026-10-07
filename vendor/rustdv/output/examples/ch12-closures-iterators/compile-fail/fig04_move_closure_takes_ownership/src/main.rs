// Rust for RTL Verification — Chapter 12, Figure 4
// "A move closure takes ownership of its captures"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0382]: borrow of moved value: `test_name`
//    --> src/main.rs:8:20
//     |
//   4 |     let banner = move || format!("*** {test_name} ***");
//     |                  ------- value moved into closure here
//   ...
//   8 |     println!("{}", test_name);
//     |                    ^^^^^^^^^ value borrowed here after move


fn main() {
    let test_name = String::from("alu_smoke_test");

    let banner = move || format!("*** {test_name} ***");

    println!("{}", banner());
    println!("{}", test_name);  // ERROR: test_name moved into the closure
}
