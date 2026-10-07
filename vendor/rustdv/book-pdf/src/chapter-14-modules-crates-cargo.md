# Chapter 14: Modules, Crates, and Cargo

We have spent thirteen chapters writing Rust in single files. Real testbenches do not live in single files: SystemVerilog testbenches grow packages and the `.f` file lists that compile them in the right order; Python testbenches grow a `tinyalu_utils.py` holding the `Ops` enum and the prediction function, imported everywhere. This chapter is about where code *lives* in Rust — modules within a project, crates between projects, and `cargo` orchestrating all of it — and it ends with a payoff I have been promising since Chapter 1: running real unit tests against testbench logic with no simulator anywhere in sight.

> **In the UVM...** we shared code through each language's namespace machinery. SystemVerilog: `import uvm_pkg::*`, `` `include "uvm_macros.svh" ``, and a `.f` file whose compilation order somebody maintains by hand. Python: an `import` statement that *ran* the module's code and added its name to our scope, `from pyuvm import *` so that Python UVM code would look like SystemVerilog UVM code, and — when Python couldn't find `tinyalu_utils` — a helping shove: `sys.path.insert(0, str(Path("..").resolve()))`.

Hold on to those last items — the hand-ordered `.f` file and the `sys.path` shove — because they are what this chapter deletes. Rust has no search path to populate, no compilation order to curate, no import-time code execution, and no possibility of a testbench that works on your machine but not on the farm because of an environment variable. What Rust has instead is more ceremonial — I will not pretend otherwise — and in exchange the compiler knows exactly where every name comes from and exactly who is allowed to use it.

## Modules: namespaces inside a crate

A **module** in Rust is a named scope you declare with the `mod` keyword. Unlike Python, where every file automatically *is* a module, a Rust module is something you declare explicitly — and you can declare one right in the middle of a file. Let's start there, because it makes the concept visible before any files get involved.

We will need a home for the TinyALU's prediction logic — the pure function that computes what the DUT *should* produce. This was `tinyalu_utils.py`'s most important resident, and it will be our example for the rest of the chapter.

```rust
// Figure 1: A module declared inline, in the middle of main.rs

mod predictor {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

    pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
        match op {
            Ops::Add => a as u16 + b as u16,
            Ops::And => (a & b) as u16,
            Ops::Xor => (a ^ b) as u16,
            Ops::Mul => a as u16 * b as u16,
        }
    }
}

fn main() {
    let sum = predictor::alu_prediction(0xFF, 0x01, predictor::Ops::Add);
    println!("0xFF + 0x01 = {sum:#06x}");
}
```

```text
--
0xFF + 0x01 = 0x0100
```

Everything between the braces of `mod predictor` lives in the `predictor` namespace, and code outside reaches it with `::` — `predictor::alu_prediction` — exactly the job SystemVerilog's `::` did in `uvm_pkg::uvm_component` (SV engineers may enjoy that Rust agrees with them about the spelling) and Python's `.` did in `pyuvm.FIFO_DEBUG`. Note in passing that the prediction itself is honest about widths in a way the Python version never had to be: the TinyALU's operands are `u8` and its result bus is sixteen bits wide, so `Add` and `Mul` cast up to `u16` *before* operating. `0xFF + 0x01` carries into bit eight instead of wrapping to zero. Chapter 3 planted that seed; here it flowers.

## `use`: bringing paths into scope

Writing `predictor::Ops::Add` at every call site gets old, and Rust's answer is the `use` declaration — the direct descendant of Python's `from ... import ...`.

```rust
// Figure 2: use brings names into scope, like Python's from-import

mod predictor {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

    pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
        match op {
            Ops::Add => a as u16 + b as u16,
            Ops::And => (a & b) as u16,
            Ops::Xor => (a ^ b) as u16,
            Ops::Mul => a as u16 * b as u16,
        }
    }
}

use predictor::{alu_prediction, Ops};

fn main() {
    println!("AND: {:#06x}", alu_prediction(0xF0, 0x3C, Ops::And));
    println!("XOR: {:#06x}", alu_prediction(0xF0, 0x3C, Ops::Xor));
}
```

```text
--
AND: 0x0030
XOR: 0x00cc
```

The mapping to your old languages is nearly one-to-one:

| SystemVerilog | Python | Rust |
|---|---|---|
| — | `import pyuvm` | *(nothing needed — see below)* |
| — | `import pyuvm as p` | `use pyuvm as p;` (aliasing works the same way) |
| `import uvm_pkg::uvm_driver` | `from pyuvm import FIFO_DEBUG` | `use pyuvm::FIFO_DEBUG;` |
| `import uvm_pkg::*` | `from pyuvm import *` | `use pyuvm::*;` (a *glob import*) |

The first row deserves a word. In Python, `import` did two jobs: it *ran the module's code* and it added a name to your scope. Rust's `use` does only the second, because there is no first — modules do not "run" when you name them. All the code in every module of your program was compiled together before the program started; `use` is purely a naming convenience, with no import-time side effects, no circular-import deadlocks, and no module-level code sneaking configuration in behind your back.¹

Rust's style community shuns glob imports for the same reason PEP-8 did — nobody reading the file can tell where a name came from. And it carves out the same exception UVM code has always carved out: crates may export a **prelude**, a curated module explicitly designed to be glob-imported. You have been using one all along — `std`'s prelude is why `String`, `Vec`, and `Option` never needed a `use`. When we reach Part II, `use rustdv::prelude::*;` will open every testbench, doing the job `import uvm_pkg::*` and `from pyuvm import *` have always done — one line, sanctioned by convention, because a testbench that starts by importing its methodology library is a testbench you can read.

> ¹ Readers who have debugged a cocotb testbench that behaved differently depending on *import order* may pause here for a private moment of celebration.

## `pub`: privacy the compiler enforces

You may have noticed the `pub` keywords sprinkled through figures 1 and 2. They are not decoration. In Rust, **everything in a module is private by default** — invisible to code outside the module — and `pub` is how an item opts into being part of the module's public interface.

Recall how your old languages handled this. Python has no private anything: every name in every module is importable by anyone, and an underscore prefix (`_my_helper`) means *please don't* — etiquette, not enforcement; pyuvm's internals are full of underscored names that nothing actually stops you from reaching. SystemVerilog has `local` and `protected` inside classes, but nothing at the package level: every name a package declares is every importer's to take. Rust makes privacy the default at every level and replaces the etiquette with a compile error. Let's provoke one. Suppose the predictor grows a private helper that the outside world has no business calling:

```rust
// Figure 3: Private by default — the underscore convention, enforced

mod predictor {
    pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

    fn widen(x: u8) -> u16 {     // no pub: private to this module
        x as u16
    }

    pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
        match op {
            Ops::Add => widen(a) + widen(b),
            Ops::And => widen(a & b),
            Ops::Xor => widen(a ^ b),
            Ops::Mul => widen(a) * widen(b),
        }
    }
}

fn main() {
    let w = predictor::widen(0xFF);  // reaching for a private helper
    println!("{w}");
}
```

```text
--
error[E0603]: function `widen` is private
  --> src/main.rs:20:24
   |
20 |     let w = predictor::widen(0xFF);  // reaching for a private helper
   |                        ^^^^^ private function
   |
note: the function `widen` is defined here
  --> src/main.rs:5:5
   |
5  |     fn widen(x: u8) -> u16 {     // no pub: private to this module
   |     ^^^^^^^^^^^^^^^^^^^^^^
```

Delete the offending line and the program compiles; `alu_prediction`, living *inside* the module, calls `widen` freely. This is encapsulation with teeth. When you mark a helper private, you are not requesting politeness — you are making a promise the compiler keeps for you: *nothing outside this module depends on this function, so I may rewrite it tomorrow without checking.* Refactoring the guts of a monitor stops requiring a grep across the testbench for who might have reached in. Note also that privacy applies to struct *fields* individually: a `pub struct` can keep its fields private, forcing outsiders through its methods — Chapter 13's gatekeeper, now standing at module scope.

## Modules in files: `mod foo;` finds `foo.rs`

Inline modules taught the concept, but real code puts modules in files. Here Rust asks for one more line of ceremony than Python did — and the line matters, so let's be precise about it.

In Python, creating `tinyalu_utils.py` *was* creating a module; the filesystem was the module system. In Rust, a file does not become part of your program until some module declares it. Writing `mod predictor;` — with a semicolon, no braces — tells the compiler: *there is a module named `predictor`, and its body lives in the file `predictor.rs` next to me.* Our playground project, reorganized:

```text
# Figure 4: The file-to-module mapping

alu_playground/
├── Cargo.toml
└── src/
    ├── main.rs          <-- contains the line: mod predictor;
    └── predictor.rs     <-- the body of the module: Ops, alu_prediction
--
% cargo run
   Compiling alu_playground v0.1.0
    Finished `dev` profile
     Running `target/debug/alu_playground`
AND: 0x0030
XOR: 0x00cc
```

Everything that was inside `mod predictor { ... }` moves into `src/predictor.rs` (dropping the wrapper braces and one level of indentation), `main.rs` declares `mod predictor;`, and nothing else changes — the `use` lines, the calls, the visibility rules are exactly as before. If `predictor` later needs child modules of its own, it graduates to a directory: `src/predictor.rs` alongside `src/predictor/history.rs`, with `mod history;` declared inside `predictor.rs`. (You will also meet an older layout, `predictor/mod.rs`, in existing code; the two styles are equivalent, and the older one's chief legacy is an editor tab bar reading `mod.rs`, `mod.rs`, `mod.rs`.²)

Why the extra declaration? Because in Rust the *module tree is part of the program*, not an emergent property of whatever files happen to be on a search path. There is no `sys.path` to populate, no `PYTHONPATH` to export on every farm machine, no `.f` file whose ordering someone curates, no "works in this directory, fails in that one." The compiler starts at the crate root — `main.rs` for a program, `lib.rs` for a library — follows the `mod` declarations outward, and compiles precisely those files. A file nobody declares is not compiled at all. It is more ceremony than Python's file-is-a-module simplicity, one honest line more per module, and what the line buys is that your program's structure is written down in the program.

> ² A directory full of files all named `mod.rs` is nobody's favorite piece of Rust history. Use the `predictor.rs`-plus-directory style in new code.

## Crates: the unit of compilation and sharing

One level up from modules sits the **crate** — the thing `cargo new` has been making for us all along. A crate is Rust's unit of compilation and distribution: the compiler compiles a crate at a time, and crates are what you publish, version, and depend on. Where Python drew a fuzzy line between "module," "package," and "distribution" (three concepts, two of which share the name *package*), and SystemVerilog's unit of sharing was a package plus the `.f` fragment that compiles it plus whatever tarball a vendor shipped it in, Rust draws one line: a crate is a tree of modules with a single root, and it compiles to a single artifact.

Crates come in two flavors you already half-know. A **binary crate** has a `main.rs` with a `fn main()` and compiles to a program — every playground so far. A **library crate** has a `lib.rs` and no `main`; it compiles to a library for other crates to use. `uvm_pkg`, cocotb, and pyuvm are the analogs of library crates; your testbench was the analog of a binary crate importing them.

When a project outgrows one crate, cargo scales up with a **workspace**: several crates developed side by side in one repository, sharing a build directory and a single lock file. This is not a distant abstraction — it is the shape of the very tools this book builds toward. `rustdv-sim` (the cocotb analog) and `rustdv` (the pyuvm analog) live as separate crates in one workspace, for the same reason cocotb and pyuvm are separate packages: you can write a Part II-style testbench against `rustdv-sim` alone, no UVM machinery in sight, and the dependency arrow only points one way. Your own testbenches, though, stay simple — one crate, depending on published ones. Which raises the question: *how* does a crate depend on another? That is `Cargo.toml`'s job.

## Cargo.toml: the manifest

Every `cargo new` wrote a `Cargo.toml` we have been politely ignoring for thirteen chapters. It is the crate's **manifest** — the one file that says what the crate is and what it needs. Time to look inside, and to add our first dependency while we're there. Python's `random` came in the standard library; Rust's standard library is deliberately lean, and random numbers live in a crate called `rand` on **crates.io**, the community registry that plays the role of PyPI. (SystemVerilog has no registry at all — sharing verification IP means tarballs and vendor portals, which is part of why every company's UVM library drifted apart.) Where Python needed `pip install` (into the right virtual environment, remember) plus a line in `requirements.txt`, cargo does both jobs with one command:

```text
# Figure 5: Adding a dependency with cargo add

% cargo add rand
    Updating crates.io index
      Adding rand v0.9.1 to dependencies
--
# Cargo.toml, after:

[package]
name = "alu_playground"
version = "0.1.0"
edition = "2024"

[dependencies]
rand = "0.9.1"
```

Two sections, both readable at sight. `[package]` names and versions the crate itself. `[dependencies]` lists what it needs — and this section *is* the requirements file, living in the project, checked into version control, impossible to forget on the farm machines. The next `cargo build` (or `run`, or `test`) downloads `rand`, compiles it, and links it; there is no separate install step, no environment to activate, and no way to run against a different version than the manifest declares. Alongside the manifest cargo maintains `Cargo.lock`, recording the *exact* versions resolved, so that every machine that builds this crate builds it with identical dependencies — the reproducibility that `pip freeze` approximated, produced automatically and kept current. With `rand` declared, using it is just a path — `rand::random::<u8>()` gives the random operands our tests are about to want. No import dance; the dependency's name is the root of its module tree, available everywhere in your crate.

## The payoff: `cargo test`

Now the capability I flagged in Chapter 1 as worth the price of admission. It has been sitting quietly inside cargo the whole time, waiting for us to have code worth testing. We do: `alu_prediction` is a pure function — values in, value out, no DUT, no signals, no simulator. In your old flows, logic like this could only be exercised by running the whole stack against a simulator; SystemVerilog cannot so much as parse your predictor without one. In Rust, testing it is built into the language and the tool. You write functions marked `#[test]`, and `cargo test` finds and runs them. The convention is a `tests` submodule at the bottom of the file whose code it tests:

```rust
// Figure 6: Unit tests live beside the code they test

// src/predictor.rs

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
    match op {
        Ops::Add => a as u16 + b as u16,
        Ops::And => (a & b) as u16,
        Ops::Xor => (a ^ b) as u16,
        Ops::Mul => a as u16 * b as u16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_carries_into_bit_eight() {
        assert_eq!(alu_prediction(0xFF, 0xFF, Ops::Add), 0x01FE);
    }

    #[test]
    fn and_masks_operands() {
        assert_eq!(alu_prediction(0xF0, 0x3C, Ops::And), 0x0030);
    }

    #[test]
    fn xor_finds_differing_bits() {
        assert_eq!(alu_prediction(0xF0, 0x3C, Ops::Xor), 0x00CC);
    }

    #[test]
    fn mul_needs_the_full_result_bus() {
        assert_eq!(alu_prediction(0xFF, 0xFF, Ops::Mul), 0xFE01);
    }
}
```

Every piece of this figure is machinery you already own. `mod tests` is an inline module, straight from figure 1. `use super::*;` is a glob import whose path `super` means "my parent module" — it pulls `Ops` and `alu_prediction` into the tests' scope, and it is the second sanctioned use of a glob import, because a test module importing everything it tests is exactly as readable as a testbench importing its methodology library. `#[cfg(test)]` tells the compiler to build this module only when compiling tests, so the shipping artifact carries no test code. And `assert_eq!` you met in Chapter 9, along with the taxonomy that governs it: assertion failures are for *bugs*, and a failed test is a bug by definition.

```text
# Figure 7: Running the unit tests

% cargo test
   Compiling alu_playground v0.1.0
    Finished `test` profile [unoptimized + debuginfo]
     Running unittests src/main.rs
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... ok
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Read that last line the way it deserves to be read: *finished in 0.00s*. Four checks of the TinyALU's prediction logic, on a laptop, in less time than a simulator takes to print its banner. No license checked out, no design elaborated, no waveform dumped — because nothing here needed one. And when a test fails, the report is a diagnosis, not a stack trace. Suppose a future refactor forgets the width lesson and writes the addition as `(a + b) as u16`, wrapping at eight bits:

```text
# Figure 8: A failing test names the culprit

% cargo test
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... FAILED
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

failures:

---- predictor::tests::add_carries_into_bit_eight stdout ----
assertion `left == right` failed
  left: 254
 right: 510

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured
```

`254` is `0xFF + 0xFF` wrapped to eight bits; `510` is the truth. The bug that would have surfaced as a scoreboard miscompare forty minutes into a regression — with the *DUT* as the initial suspect — instead surfaced in milliseconds, correctly attributed to the predictor, before any simulator ran.

Savor what just changed, because it is a new capability, not a nicer version of an old one. Your testbench's pure logic — predictors, transaction arithmetic, coverage binning, anything that computes without touching a signal — now has its own test suite that runs on every build, for free. Your old stacks kept all testbench verification inside the simulation; the testbench was only ever as tested as your last regression. From here on, this book runs `cargo test` habitually: when the later chapters build scoreboards and coverage collectors, their logic arrives with unit tests beside it, and the simulator's time is spent on the only thing that actually needs a simulator — the DUT.

## Summary

In this chapter we gave Rust code a place to live. `mod` declares modules — inline for small things, `mod foo;` pointing at `foo.rs` for real ones — and the module tree is written in the program rather than discovered on a search path. `use` brings paths into scope the way `from ... import` did, glob imports and all, with preludes as the sanctioned exception. Everything is private until `pub` says otherwise, turning Python's underscore etiquette into a compiler-kept promise. Crates are the unit of compilation and distribution — Python's module/package/distribution muddle, resolved into one concept — with workspaces gathering related crates, as rustdv's own crates will be gathered. `Cargo.toml` declares dependencies, `cargo add` fetches them from crates.io, and `Cargo.lock` makes every build reproducible. And `cargo test` runs unit tests against pure testbench logic in milliseconds, no simulator required — a capability we will never stop using.

This chapter also closes Part I, so take a step back and look at what you now own. You came in knowing no Rust. You now hold the whole toolkit this book's testbenches are built from: ownership and borrowing, the responsibility-and-access model that replaces the garbage collector and turns data races into compile errors; structs, enums, and `match`, which gave `Ops` and four-state logic honest types; `Result` and `Option`, where exceptions and sentinels used to be; traits, doing the work of inheritance, the dunder methods, and `do_compare`; generics, finishing what parameterized classes started; closures and iterators, which will carry the factory's job; the smart pointers that make sharing explicit; and now modules, crates, and cargo to organize and test all of it. Every one of these landed on ground your UVM experience prepared, and every one was chosen because a coming chapter needs it. What you cannot yet do is *wait* — no timer, no rising edge, no way to say "pause this task until something happens in the simulation." That is Part II's business. Chapter 15 takes up the question every testbench language must answer — how does software wait for hardware? — and shows how `async`/`await` works when the language gives you the syntax but hands *you* the engine. The simulator is finally in sight.
