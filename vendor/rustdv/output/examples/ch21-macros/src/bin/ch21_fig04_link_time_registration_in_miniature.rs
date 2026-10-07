// Chapter 21, Figure 4: Link-time registration in miniature
// Run: cargo run --bin ch21_fig04_link_time_registration_in_miniature

use linkme::distributed_slice;

/// What a registration carries: a name and a function to run.
struct Registration {
    name: &'static str,
    run: fn(),
}

#[distributed_slice]
static TESTS: [Registration];

fn hello() {
    println!("Hello, world.");
}
#[distributed_slice(TESTS)]
static REG_HELLO: Registration = Registration { name: "hello", run: hello };

fn goodbye() {
    println!("Goodbye, world.");
}
#[distributed_slice(TESTS)]
static REG_GOODBYE: Registration = Registration { name: "goodbye", run: goodbye };

fn collect() -> &'static [Registration] {
    &TESTS
}

fn main() {
    let tests = collect();
    println!("found {} registered tests:", tests.len());
    for t in tests {
        print!("  {} -> ", t.name);
        (t.run)();
    }
}
