// Rust for RTL Verification — Chapter 12, Figure 8
// "The Fibonacci generator, as an Iterator implementation"
// Run with: cargo run --bin ch12_fig08_fibonacci_generator_iterator_implementation
//
// Expected output:
//   0 1 1 2 3 5 8 13


struct Fibonacci {
    curr: u64,
    next: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let result = self.curr;
        self.curr = self.next;
        self.next = result + self.next;
        Some(result)
    }
}

fn main() {
    let fib = Fibonacci { curr: 0, next: 1 };
    for numb in fib.take(8) {
        print!("{numb} ");
    }
    println!();
}
