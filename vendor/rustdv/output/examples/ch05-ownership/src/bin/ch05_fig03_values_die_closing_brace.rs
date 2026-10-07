// Rust for RTL Verification — Chapter 5, Figure 3
// "Values die at the closing brace — every time, on time"
// Run with: cargo run --bin ch05_fig03_values_die_closing_brace
//
// Expected output:
//   inside the scope: MUL 7 6
//   after the scope


fn main() {
    {
        let cmd = String::from("MUL 7 6");
        println!("inside the scope: {cmd}");
    }   // <- cmd's owner goes out of scope RIGHT HERE.
        //    The String is dropped, its memory freed, before
        //    the next line runs. No collector. No "eventually."
    println!("after the scope");
}
