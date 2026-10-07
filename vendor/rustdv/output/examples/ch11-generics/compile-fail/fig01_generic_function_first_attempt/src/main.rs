// Rust for RTL Verification — Chapter 11, Figure 1
// "A generic function, first attempt — the compiler wants proof"
// NOTE: this figure FAILS TO COMPILE ON PURPOSE — the error is the lesson.
// Build it and read the error: cargo build   (see EXPECTED.txt)
//
// Expected compiler error:
//   error[E0369]: binary operation `>` cannot be applied to type `&T`
//    --> src/main.rs:4:17
//     |
//   4 |         if item > largest {
//     |            ---- ^ ------- &T
//     |            |
//     |            &T
//     |
//   help: consider restricting type parameter `T`
//     |
//   1 | fn largest<T: std::cmp::PartialOrd>(list: &[T]) -> &T {
//     |             ++++++++++++++++++++++


fn largest<T>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}
