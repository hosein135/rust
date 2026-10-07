# Chapter 1: Why Rust?

*Rust for RTL Verification* is a book for verification engineers who want to write testbenches in Rust. It teaches the language from zero — not one line of prior Rust is assumed — and then uses rustdv, a UVM verification framework written in Rust, to build a complete, running testbench for a small ALU. If you have written UVM testbenches in any language, this book was written for you.

Verification engineers come to Rust from two directions. Most write SystemVerilog UVM testbenches and have spent years with `uvm_config_db`, `type_id::create()`, and the sequencer handshake. A growing number write Python testbenches with cocotb and pyuvm, and know the same methodology by its Python names. This book addresses both of you, usually at the same time, because you share the thing that matters: you know what a driver is for, why a scoreboard subscribes to a monitor, and what the factory buys you. That shared knowledge — the UVM, not any particular language — is the ground this book builds on.

What the book assumes, then, is verification: the UVM's concepts and vocabulary. If you have that from work, you are ready. If you want to build it first, two earlier books in this series teach it from first principles: [*The UVM Primer*](https://www.uvmprimer.com) in SystemVerilog and [*Python for RTL Verification*](https://a.co/d/0hTKAJvh) in Python. Either one prepares you for this book. Neither is required.

A note on what you do *not* need: any Rust. Not one line. If you have heard alarming rumors about a thing called the borrow checker, you have heard correctly, and we will make friends with it in Chapters 5 and 6.

## Two revolutions

The first revolution is the one you lived through, or inherited. In the 1990s, verification engineers realized that testbenches are not test fixtures but *software* — software that happens to talk to a simulated design. That realization gave us the verification languages (e, Vera, SUPERLOG), then SystemVerilog, then a decade of methodology wars that ended with every EDA vendor blessing a single winner: the Universal Verification Methodology. Later the same realization pushed further — if testbenches are software, why not write them in a general-purpose software language? — and produced cocotb and pyuvm, which moved testbenches off the simulator and into Python. Every step followed the same moral: testbenches are software, and software deserves a software language.

Rust's story rhymes with it. In 2006, a Mozilla engineer named Graydon Hoare started a personal project to answer an uncomfortable question: why, decades into the software era, were our foundational programs — browsers, kernels, the code we bet everything on — still written in languages that let one stray pointer corrupt everything? C and C++ were fast because they trusted the programmer completely, and every security bulletin showed what that trust cost.¹

The conventional answer was garbage collection: let a runtime babysit memory, and accept the slowdown. That is Python's answer, and SystemVerilog's too — class objects in SV live until the last handle drops, collected automatically, exactly like Python objects. Rust proposed a third answer: what if the *compiler* proved memory safety, at compile time, and the finished program paid nothing at all? No garbage collector, no interpreter, no runtime babysitter. The rules that make this possible — ownership and borrowing — are the subject of Chapters 5 and 6, and they will bend your brain exactly once, after which you will wonder how you ever tracked object lifetimes in your head.

Rust 1.0 shipped in 2015. Since then the language has spent year after year at the top of developer-survey "most admired" lists, and it has done something no other language managed in half a century: it convinced the Linux kernel, Windows, and Android teams to admit a second systems language into their codebases. That is not fashion. That is an industry deciding that memory safety without a garbage collector is worth learning something hard.

> ¹ The security community eventually put numbers on it: both Microsoft and Google's Chrome team reported that roughly 70% of their serious security bugs were memory-safety bugs — the exact category Rust eliminates at compile time.

## Why Rust for verification?

It is fair to ask why a verification engineer with a working flow should care. Rust's compiler earns its keep on data and ownership. Transactions are plain structs whose fields the compiler knows by name: rename a field, and it produces the complete list of every place that must change, and the testbench does not build until you have addressed all of them. Every object has exactly one owner: hand a transaction to the driver, and the compiler knows you no longer have it, which settles at compile time a question — who may still touch this object? — that verification methodologies otherwise answer with convention and code review. These checks pay off in proportion to the size of the testbench and the number of hands on it. A fifty-component environment refactored by four people gets something from them that a five-hundred-line testbench does not need.

Configuration, the factory, and TLM connection, on the other hand, resolve at run time in rustdv — deliberately, because that is what late binding *is*. The whole point of those three layers is to defer decisions so that one environment serves many tests, and a compiler can only check what is known before the program runs. A wrong configuration key, a missing factory override, an unconnected port: these fail at run time in rustdv as they do in every UVM, and the chapters that teach each layer show exactly what the failure looks like.

The other reason to care is throughput. Rust compiles to the same kind of native code as the simulator itself, with no interpreter and no garbage collector. If you come from Python, you know that every signal read, every transaction compare, every scoreboard update runs through the interpreter; for a small ALU it does not matter, but for a regression farm running thousands of seeds against a large SoC, testbench overhead is real money and real schedule. And a testbench with no collector pauses and no interpreter in the loop is the kind of testbench that can keep up with an emulator. This argument is about speed, not correctness — but speed is a verification resource like any other.

The rest of the ledger is smaller but real:

**Unit testing without a simulator.** Rust testbench components are ordinary structs, so the pure-software parts of your testbench — predictors, transaction operations, coverage logic — can be tested with `cargo test` in milliseconds, on your laptop, with no simulator license anywhere in sight. If you have ever queued for a license to test a scoreboard change that never touches a signal, this feature alone may justify the book.

**One binary, no environment.** A Rust testbench compiles to a single library that the simulator loads. There is no interpreter version to match, no virtual environment to activate, no `pip install` on the farm machines. If it built, it runs.

**An open toolchain.** The Rust compiler, the cargo build system, the package ecosystem, and rustdv itself are free and open source. The simulator is the only licensed tool left in the loop — and this book's examples run on Icarus Verilog, which is also free.

## What it costs

I owe you the other side of the ledger, because there is one.

Rust is hard to learn. Not a little hard — the ownership system is a new idea, and for your first weeks the compiler will reject code you are certain is fine. (It is almost never fine. This is the maddening part. Later it becomes the endearing part.) If you come from Python, you are trading a language that works hard to be unsurprising for one that holds opinions and holds them at compile time. If you come from SystemVerilog, take heart: you have already mastered one of the largest languages in engineering, and Rust is smaller, more consistent, and better documented than what you already know. Different, though. Chapter 2 maps what carries over and what must be unlearned, for both of you.

The edit-run loop includes a compile. For testbench work the compile is usually seconds, not minutes, but the rhythm is different from an interpreted flow and you will feel it.

The verification ecosystem is younger. SystemVerilog has two decades of UVM infrastructure; Python has cocotb, pyuvm, and years of conference papers; Rust verification is early. That is part of why this book exists — someone gets to write the early chapters of that story, and it may as well be us.

If your testbenches are small, your regressions short, and your team fluent in its current language, that language remains a fine answer, and I will not pretend otherwise. This book is for when one of those stops being true.

## Code examples

Every example has a figure number; code is followed by `--` and then its output, and every transcript in this book is genuine tool output. You can get a copy of the examples from the book's repository, organized in directories named after their chapters, each with a `README.md` explaining how to run it. Early chapters use small standalone cargo projects as playgrounds; from Chapter 15 on, examples are simulation directories.

Our running example is the TinyALU: a two-input ALU with a `start`/`done` handshake, small enough that the testbench, not the design, stays the subject. Readers of the earlier books will recognize it; Chapter 18 specifies it fully, so newcomers lose nothing. By the final chapter you will have built its testbench eight times, versions 1.0 through 8.0, each version adding one architectural idea — the same climb the earlier books made.

We begin where every programming book begins. In figure 1, we create a program with `cargo new`, Rust's project generator — meet `cargo` now, because it is the package manager, build system, test runner, and project generator fused into one tool, and it will be everywhere.

```text
# Figure 1: Creating our first program

% cargo new hello
    Creating binary (application) `hello` package
```

`cargo new` writes a tiny project containing `src/main.rs`, which is where figure 2 lives. Rust asks even the smallest program for a function — `fn main()` is where every Rust program begins. The exclamation point on `println!` marks it as a *macro* rather than a function, a distinction that will matter a great deal in Chapter 21 and not at all before then.

```rust
// Figure 2: The classic first program

fn main() {
    println!("Hello, world.");
}
```

```text
% cargo run
   Compiling hello v0.1.0
    Finished `dev` profile
     Running `target/debug/hello`
Hello, world.
--
Hello, world.
```

Notice the compile between `cargo run` and the greeting. That step is the new resident in your edit-run loop, and whether it earns its rent is the question the rest of this book answers with running testbenches.

## The plan

Part I (Chapters 1–14) teaches the Rust you need, always against the languages you know: ownership where they had automatic collection, traits where they had inheritance, `Result` where they had exceptions or error signals, `match` where they had `case` and `if` chains.

Between the two parts sits an Interlude: the complete TinyALU testbench, presented whole and unexplained. It is your first sight of the destination — read it the way you would walk through a finished house before studying the blueprints.

Part II (Chapters 15–40) climbs to it. First the machinery: async/await — the same coroutine idea that powers both SystemVerilog's tasks and cocotb's scheduler, except that Rust hands you the engine — then tasks and queues, then the simulator, then testbenches 1.0 and 2.0, then one load-bearing chapter on macros. From Chapter 22 the UVM is rebuilt piece by piece: tests, components and the phase lifecycle, environments, logging, configuration, the factory, component communication, transactions, and sequences, through testbench versions 3.0 to 8.0. Chapter 40 returns to the Interlude's testbench and walks it line by line, with everything explained. The appendices map this book's chapters onto the two earlier books and collect the idiom translations from Python and SystemVerilog.

The TinyALU is waiting. First, the language.
