# Rust for RTL Verification

**A complete course in Rust, rustdv-sim, and rustdv** — full chapter outline plus drafted Chapter 1

| | |
|---|---|
| Companion to | *Python for RTL Verification* (Salemi, 2022) |
| Design basis | `.design-doc.md` (this repository); cocotb @ `cf833ee`; pyuvm @ `dfcd1ff` |
| Status | Outline + Chapter 1 draft, for review |

---

## Premise and structure

*Python for RTL Verification* had a two-audience promise: verification engineers who wanted Python, and Python programmers who wanted the UVM. This book has a sharper premise: **the reader has read the Python book** (or knows cocotb and pyuvm cold) **and knows no Rust.** Every concept lands on ground the Python book prepared. Rather than teach Rust in the abstract and verification afterward, the book is sequenced as a Rust-learning path where each language concept arrives exactly when the testbench needs it: ownership before the component hierarchy, traits before components, async before triggers, closures before variation points.

The book keeps the Python book's conventions exactly: the TinyALU as the single running example; testbench versions 1.0 through 8.0 with the same version numbers meaning the same architectural steps; numbered figures per chapter in the `# Figure N:` style with `--` separating code from output; Jupyter-style standalone examples early (here: `cargo` playground projects), simulation directories later; wry footnotes permitted.¹

A recurring sidebar device, **"In Python we..."**, opens most chapters by quoting the pattern from the Python book that the chapter re-builds in Rust. The reader should feel the two books lying open side by side.

> ¹ Encouraged, actually.

---

## Chapter outline

### Part I — Rust for the Python-fluent (mirrors "Python concepts" → "Design patterns")

**Chapter 1: Why Rust?** *(drafted in full below)*
The case for a compiled, statically-typed systems language in verification; what Python taught us and where its ceiling is; the honest costs; hello world with cargo; how to read this book. Mirrors: Introduction / "Why Python and why UVM?".

**Chapter 2: Rust concepts**
The compiler as collaborator rather than adversary; compile-time vs. runtime thinking; no interpreter, no GC, no GIL; editions and toolchain (`rustup`, `cargo`, `rustfmt`, `clippy`); reading compiler errors as a skill (they will be our best teacher, and the book leans on real error messages as figures). Mirrors: "Python concepts".

**Chapter 3: Rust basics**
`let`, immutability by default, shadowing; scalar types and *why the sizes matter now* (the TinyALU's A leg really is a `u8`); `println!` and format strings; expressions vs. statements; functions. Figures port the Python book's basics examples one-for-one so the reader can diff the languages. Mirrors: "Python basics".

**Chapter 4: Conditions, loops, and `match`**
`if`/`else` as expressions; `loop`/`while`/`for`; ranges; `match` as the load-bearing construct Python never had — introduced early because the rest of the book uses it constantly (on `Result`, on `Option`, on `Ops`). Mirrors: "Conditions and loops" + "Ranges".

**Chapter 5: Ownership**
The chapter with no Python-book mirror — and the hinge of the whole language. Move semantics; scope-based drop; clone vs. move; the "who frees this?" question Python's GC answered silently. Taught with transaction objects: what *should* happen when a monitor hands a transaction to a scoreboard? Ends with the mental model used for the rest of the book: *ownership is about responsibility, borrowing is about access*.

**Chapter 6: Borrowing and references**
`&T` and `&mut T`; aliasing XOR mutability; lifetimes only as far as testbench code needs them (deliberately shallow — same instinct as the Python book skipping metaclasses). Revisits the Python book's NullTrigger race example and shows the borrow checker rejecting it at compile time.

**Chapter 7: Structs, enums, and methods**
Structs vs. Python classes; `impl` blocks; associated functions vs. methods; **enums with payloads** — the star of the chapter. `Ops` returns from the Python book's `tinyalu_utils`, now as a real sum type; `Logic` (0/1/X/Z) as a four-state enum. Mirrors: "Classes" + the `Ops` IntEnum discussion in "Basic testbench: 1.0".

**Chapter 8: Collections: `Vec`, `String`, and `HashMap`**
The Python sequences chapters compressed into one, because the concepts transfer readily; emphasis on what's different: ownership *inside* collections, `&str` vs. `String`, iteration borrowing. Mirrors: "Python sequences" / "Lists" / "Strings" / "Dictionaries" / "Commonly used sequence operations".

**Chapter 9: `Result`, `Option`, and the end of exceptions**
`Option` for absence, `Result` for fallibility, `?` for propagation, `panic!` for bugs; designing error enums. Ports the Python book's exceptions chapter scenario-for-scenario, then establishes the testbench failure taxonomy the design doc fixed: `Err` for checks, panic for testbench bugs. Mirrors: "Exceptions".

**Chapter 10: Traits**
Interfaces without inheritance; default methods; deriving; operator traits (`PartialEq` replaces `__eq__`, `Display` replaces `__str__` — mapped explicitly to the dunder methods the Python book taught); trait objects vs. generics, and when each appears in rustdv. Mirrors: "Inheritance" + "The super() function" + protocols material.

**Chapter 11: Generics**
Type parameters and bounds; monomorphization (why generic code costs nothing at runtime); `Driver<REQ, RSP>` previewed as the destination. Mirrors: the duck-typing discussions throughout the Python book's class chapters.

**Chapter 12: Closures and iterators**
`|x| x + 1`; capture modes through the ownership lens; iterator adapters replacing comprehensions; `impl Iterator` returns replacing generators. Closures as *values* you can store in structs and pass to constructors — planted here as the seed of Chapter 29's variation points. Mirrors: "Generators" + comprehension material.

**Chapter 13: Smart pointers: `Box`, `Rc`, `RefCell`**
The escape hatches, taught honestly: when shared mutability is the right design and what it costs; `Rc<RefCell<T>>` as "a Python reference, made visible." Placed here because Part IV needs the vocabulary — `Rc<TinyAluBfm>` shared through config structs, and the explanation of why the ownership-tree hierarchy avoids `Rc<RefCell>` cycles entirely. Mirrors: "Protecting attributes" (both are chapters about controlled access).

**Chapter 14: Modules, crates, and cargo**
`mod`/`use`/visibility; workspace layout; `Cargo.toml`; adding dependencies; `cargo test` — the reader runs real unit tests before ever touching a simulator, a genuinely new capability worth savoring. Mirrors: "Modules".

### Part II — Concurrency and simulation (mirrors "Coroutines" → "Class-based testbench: 2.0")

**Chapter 15: `async`/`await` and the executor**
The chapter that pays off the Python book's "Coroutines" chapter. Same Rogue-game event-loop framing, then: futures as state machines, `poll`, wakers, and *why Rust ships the syntax but not the loop*; cocotb wrote its own event loop and so does rustdv — the reader has seen this movie. Timer example ported verbatim (VHDL and SystemVerilog comparison figures retained). Mirrors: "Coroutines".

**Chapter 16: Tasks, channels, and sim-aware queues**
`spawn` and `TaskHandle` (mapping `start_soon`, the seven task states, awaiting a task); drop-based cancellation vs. `kill()` — the one place the mental model genuinely diverges, taught with care and the shutdown-message idiom; `sim::Queue`, `Event`, `Lock` with the fairness guarantee; producer/consumer example ported. Mirrors: "cocotb Queue" + task material in "Coroutines".

**Chapter 17: Simulating with rustdv-sim**
First simulation. The GPI lineage (same C layer under both books — a nice continuity story); getting the DUT handle; `child()` lookups returning `Result` (vs. `dut.signal` magic — and why the magic couldn't come along); reading and writing signals; `Deposit` vs. `Force`; triggers: `Timer`, edges, `ReadOnly`/`ReadWrite`; the clock. Mirrors: "Simulating with cocotb".

**Chapter 18: Basic testbench: 1.0**
The TinyALU returns. Same DUT, same protocol, same simple loop testbench, now in Rust: random operands, prediction function, result check. The chapter's teaching payload is how `Result` and the `?` operator shape testbench code. Mirrors: "Basic testbench: 1.0".

**Chapter 19: TinyAluBfm**
The BFM pattern ported: struct owning typed handles, spawned driver/monitor loops, queue-fed async API (`send_op`, `get_cmd`, `get_result`); interior state without a singleton — and a sidebar on why the Python singleton was doing ownership work that Rust does in the type system. Mirrors: "TinyAluBfm".

**Chapter 20: Struct-based testbench: 2.0**
Testbench 2.0: driver/monitor/scoreboard as structs with methods, wired manually. Deliberately shows the pain (plumbing, shared access to the scoreboard) that Part IV's machinery will remove — same rhetorical job as the Python book's 2.0 chapter. Mirrors: "Class-based testbench: 2.0".

### Part III — Macros (new material, positioned deliberately)

**Chapter 21: Macros: code that writes code**
Declarative macros briefly; then attribute and derive macros as *the* replacement for decorators and metaclasses, with `@cocotb.test()` → `#[rustdv::test]` as the worked example — including what each actually does, side by side, at import time vs. compile time. Explains link-time registration (how tests get collected with no import step — and why components, unlike tests, need no registry at all). Placed immediately before the UVM part because Part IV leans on `#[derive(Component)]` and the std derives. Mirrors: the decorator material in "Functions"/"Design patterns", elevated to a full chapter because Rust makes the machinery visible.

### Part IV — The UVM in Rust (mirrors "Why UVM?" → "Virtual sequence testbench: 8.0", one-for-one)

**Chapter 22: Why UVM?**
The Python book's argument restated for a new language — plus the new question this book must answer: *does a statically-typed language change what the UVM is for?* (Answer: no — reuse and methodology arguments survive intact; some of the UVM's runtime machinery becomes compile-time machinery, and that's a feature.) Mirrors: "Why UVM?".

**Chapter 23: uvm_test testbench: 3.0**
Testbench 3.0: the first `rustdv` (UVM-analog) test. The `#[rustdv::test]` function *is* the test — it constructs and owns the env (no `uvm_root`, no `run_test()` string dispatch); the lifecycle arrives in minimal form (`start`, `check`, `report`); objection guards (RAII vs. raise/drop — the guard pattern taught here and used everywhere after). Mirrors: "uvm_test testbench: 3.0".

**Chapter 24: Components: the hierarchy problem, solved by ownership**
The problem `uvm_component` solves — structured, reusable testbench composition — and its Rust dissolution: the ownership tree *is* the component tree. Children as struct fields, `#[derive(Component)]` traversal, hierarchical names from field names, and what `self.parent.thing` becomes (field access, it turns out). The predefined component taxonomy (env/agent/driver/monitor/scoreboard/subscriber) introduced. Mirrors: "uvm_component".

**Chapter 25: uvm_env testbench: 4.0**
Testbench 4.0: structure via a plain env struct whose fields are its children; the build and connect *conventions* — constructors construct, constructor arguments wire — with lifecycle traversal orders shown in log output, as the Python book did. Mirrors: "uvm_env testbench: 4.0".

**Chapter 26: Logging**
`tracing` targets and levels mapped onto the component hierarchy; per-component log levels (`set_logging_level_hier` equivalent); the sim-time-stamped format matched to the Python book's output so figures stay comparable. Mirrors: "Logging".

**Chapter 27: Configuration: the ConfigDB problem, solved by types**
The problem the ConfigDB solves — tests parameterizing components buried deep in a hierarchy they didn't write — and its Rust dissolution: nested config structs mirroring the ownership tree, built by the test, checked by the compiler. Sharing resources (`Rc<TinyAluBfm>`) as plain fields. Every ConfigDB failure mode from the Python book, replayed as a compile error. Mirrors: "ConfigDB()".

**Chapter 28: Configuration debugging: what the compiler now does for you**
The Python book needed a chapter to debug the ConfigDB; this one walks the same classic mistakes — wrong path, wrong phase, shadowed precedence, wrong type — and shows where each went: runtime mysteries become compile errors or impossibilities. What debugging remains (wrong config *values* rather than wrong config *plumbing*) and how to read the compiler's messages when a config tree doesn't build. Mirrors: "Debugging the ConfigDB()".

**Chapter 29: The factory problem, solved by closures and generics**
What the factory is *for* — tests changing testbench behavior without editing the env — and the Rust mechanism that delivers it: constructors as values, maker closures carried in config structs, explicit variation points instead of a global registry with override chains. Where a string registry rightly survives (test discovery — callback to Chapter 21) and the honest loss (instance-path-pattern overrides). Mirrors: "The UVM factory".

**Chapter 30: Variation-point testbench: 5.0**
Testbench 5.0: the random-ops test becomes max-ops without touching the env — by handing the env a different sequence and a different maker closure, three visible lines in the test. Same demonstration as the Python book's 5.0, new mechanism, same version number. Mirrors: "UVM factory testbench: 5.0".

**Chapter 31: Component communications**
What TLM-1's thirty classes were *for*, and the two types that do the job in Rust: `Sender<T>`/`Receiver<T>` channels — blocking and nonblocking send/recv/peek as six methods; `TlmFifo<T>` for when the FIFO belongs in the hierarchy; wiring as constructor arguments. Connection mistakes as *compile errors* (figure: the actual compiler message where the Python book showed a runtime `UVMTLMConnectionError`). Mirrors: "Component communications".

**Chapter 32: Analysis ports**
1-to-many `write()`; `Subscriber<T>` trait; analysis FIFOs; building the monitor→scoreboard/coverage fan-out. Mirrors: "Analysis ports".

**Chapter 33: Components in testbench 6.0**
Testbench 6.0, part one: Driver (typed `SeqItemPort` preview), Monitor(s), Scoreboard, Coverage as real components. Mirrors: "Components in testbench 6.0".

**Chapter 34: Connections in testbench 6.0**
Testbench 6.0, part two: wiring by constructor argument, end to end (the connect convention at full scale); the agent with active/passive from its config — `Option` children mean a passive agent simply has no driver to misuse. Mirrors: "Connections in testbench 6.0".

**Chapter 35: Transactions: the uvm_object problem, solved by std derives**
What `uvm_object` gave Python — field-wise copy, compare, and printing — and why Rust transactions need none of it: `#[derive(Clone, Debug, PartialEq)]` on a plain struct. The dunder-method table from the Python book's chapter returns with a third column that is mostly the word *derive*. Where comparison policy went (the scoreboard, as a comparator) and where transaction identity went (the `SeqItem` envelope, owned by the sequencer — setting up Chapter 38). Mirrors: "uvm_object in Python".

**Chapter 36: Sequence testbench: 7.0**
Testbench 7.0: the full sequence machinery — `Sequence` trait and `body`; `start_item`/`finish_item`; driver's `get_next_item`/`item_done`; the handshake walked through event-by-event with the same diagrams as the Python book. Mirrors: "Sequence testbench: 7.0".

**Chapter 37: Fibonacci testbench: 7.1**
Testbench 7.1: sequences with state feeding back; task lifetime and the drop-based-cancellation idiom in a real testbench. Mirrors: "Fibonacci testbench: 7.1".

**Chapter 38: get_response testbench: 7.2**
Testbench 7.2: transaction IDs carried by the `SeqItem` envelope — where the Python book needed `set_context`, the infrastructure now tags responses itself — and cherry-picking responses by id. Mirrors: "get_response() testbench: 7.2".

**Chapter 39: Virtual sequence testbench: 8.0**
Testbench 8.0: coordinating sub-sequences; virtual sequences as the type-system version (no item channel — misuse doesn't compile, where pyuvm raised `UVMSequenceError`). Mirrors: "Virtual sequence testbench: 8.0".

### Part V — Capstone and closing

**Chapter 40: The complete TinyALU testbench**
The capstone the Python book never needed: the full 8.0-era testbench presented end-to-end in one chapter — project layout, every file, build and run with `rustdv run`, reading the regression report, plus cargo unit tests for the pure-Rust pieces. Serves readers who will use the book as a reference template.

**Chapter 41: The future of Rust in verification**
Where this goes: pure-Rust GPI, multi-core potential beyond the single-threaded model, the register abstraction layer, growing an open-source ecosystem; a frank assessment mirroring the Python book's closing chapter. Mirrors: "The future of Python in verification".

**Back matter:** appendix mapping every Python-book chapter to its companion chapter(s); appendix of Python→Rust idiom translations (the design doc's §1 table, reader-formatted); index.

---
---

# Chapter 1: Why Rust? — full draft

Rust for RTL Verification is a book for verification engineers who have outgrown an interpreter and for Rust programmers who want to learn the Universal Verification Methodology (UVM). Mostly, though, it is a book for readers of *Python for RTL Verification* who are ready for a second language — one that trades a little of Python's ease for a lot of speed and an entirely new superpower: a compiler that finds testbench bugs before the simulator ever runs.

This book teaches you Rust the way the last book taught you Python: just enough of the language, arriving just in time, to build testbenches with rustdv-sim (our cocotb equivalent) and rustdv (our pyuvm equivalent). By the final chapter you will have rebuilt the TinyALU testbench — the same TinyALU, the same testbench architecture, versions 1.0 through 8.0 — in a language that compiles to native code and races through regressions.

## The book assumes you know the Python story

*Python for RTL Verification* assumed you knew how to program. This book assumes more: that you know how to program *testbenches*, the way that book taught them. When we meet a sequence in Chapter 36, I will not explain what a sequence is for — you know. I will explain what it looks like in Rust and why it looks that way. If you have not read the Python book but know cocotb and pyuvm well, you will be fine. If neither is true, read that book first; this one will still be here.

A note on what you do *not* need: any Rust. Not one line. If you have heard alarming rumors about a thing called the borrow checker, you have heard correctly, and we will make friends with it in Chapter 5.

## Two revolutions, thirty years apart

The Python book told the story of how we came to verify hardware with software — from e and SUPERLOG through the methodology wars to the UVM, and then sideways, off the simulator entirely, to Python. That story had a moral: testbenches are software, and software deserves a software language.

Rust's story rhymes with it. In 2006, a Mozilla engineer named Graydon Hoare started a personal project to answer an uncomfortable question: why, decades into the software era, were our foundational programs — browsers, kernels, the code we bet everything on — still written in languages that let one stray pointer corrupt everything? C and C++ were fast because they trusted the programmer completely, and every security bulletin showed what that trust cost.²

The conventional answer was garbage collection: let a runtime babysit memory, and accept the slowdown. That is Python's answer, and for testbenches it is a fine one — until it isn't. Rust proposed something genuinely new: what if the *compiler* proved memory safety, at compile time, and the finished program paid nothing at all? No garbage collector, no interpreter, no runtime babysitter. The rules that make this possible — ownership and borrowing — are the subject of Chapter 5, and they will bend your brain exactly once, after which you will wonder how you ever tracked object lifetimes in your head.

Rust 1.0 shipped in 2015. Since then the language has spent year after year at the top of developer-survey "most admired" lists, and it has done something no other language managed in half a century: it convinced the Linux kernel, Windows, and Android teams to admit a second systems language into their codebases. That is not fashion. That is an industry deciding that compile-time correctness is worth learning something hard.

> ² The security community eventually put numbers on it: both Microsoft and Google's Chrome team reported that roughly 70% of their serious security bugs were memory-safety bugs — the exact category Rust eliminates at compile time.

## Why Rust for verification?

It is fair to ask why a verification engineer with a working Python flow should care. I will give you the honest engineering answer in four parts.

**Speed.** Python is an interpreted language, and for all of cocotb's cleverness, every signal read, every transaction compare, every scoreboard update runs through the interpreter. For the TinyALU it does not matter. For a regression farm running thousands of seeds against a large SoC, testbench overhead is real money and real schedule. Rust compiles to the same kind of native code as the simulator itself. When the testbench stops being the bottleneck, you buy back regression time without touching your license count.

**Correctness before simulation.** This is the deeper reason, and the one this book keeps returning to. In Python, a typo'd signal name, a transaction handed to the wrong port, a scoreboard mutated by two tasks at once — all of these are discovered *at runtime*, which in our world means *after the simulator license is checked out, the design is elaborated, and forty minutes have passed*. Rust moves an astonishing fraction of these discoveries to the compile step, which takes seconds and costs nothing. A TLM connection mistake that pyuvm reports as a runtime `UVMTLMConnectionError` simply does not compile in rustdv. The Python book warned you about a race between two tasks sharing `transaction_data` and taught you to avoid it by discipline. The Rust compiler *rejects that code*. Discipline is good; proof is better.

**Testbench refactoring without fear.** Verification code lives longer than we admit and gets modified by more hands than we would like. In Python, renaming a field means grepping and praying; the interpreter will tell you what you missed, one AttributeError at a time, over the following month. In Rust, the compiler produces the complete list of every place that must change, and the testbench does not build until you have addressed all of them. This changes how boldly you can improve old testbenches.

**One binary, no environment.** A Rust testbench compiles to a single library that the simulator loads. There is no interpreter version to match, no virtual environment to activate, no `pip install` on the farm machines. If it built, it runs.

And one more, which deserves its own paragraph: **unit testing without a simulator.** Because Rust testbench components are ordinary structs, the pure-software parts of your testbench — predictors, transaction operations, coverage logic — can be tested with `cargo test` in milliseconds, on your laptop, with no simulator license anywhere in sight. The Python stack could do some of this; the Rust toolchain makes it so frictionless that this book does it constantly, starting in Chapter 14.

## What it costs

I owe you the other side of the ledger, because there is one.

Rust is harder to learn than Python. Not a little harder — the ownership system is a genuinely new idea, and for your first weeks the compiler will reject code you are certain is fine. (It is almost never fine. This is the maddening part. Later it becomes the endearing part.) The Python book could teach its whole language in fourteen short chapters because Python works hard to be unsurprising; Rust holds opinions, and it holds them at compile time.

The edit-run loop is slower. Python starts instantly; Rust compiles first. For testbench work the compile is usually seconds, not minutes, and it replaces the forty-minute runtime failure — but the rhythm is different and you will feel it.

The verification ecosystem is younger. Python has cocotb, pyuvm, and years of conference papers; Rust verification is early. That is part of why this book exists — someone gets to write the early chapters of that story, and it may as well be us.

There is no REPL. Python let you poke at ideas interactively; Rust asks you to write a small program. The playground projects in Part I are this book's substitute, and honestly, `cargo` makes them cheap enough that you will not miss the prompt much.

If your testbenches are small, your regressions short, and your team fluent in Python, Python remains a fine answer, and I will not pretend otherwise. This book is for when one of those three stops being true.

## Code examples

The conventions are the ones you know. Every example has a figure number; code is followed by `--` and then its output. Examples live in the `rust4uvm_examples` repository, in directories named after their chapters, with instructions in each `README.md`. Early chapters use standalone cargo projects the way the Python book used Jupyter notebooks; from Chapter 17 on, examples are simulation directories.

We should not break a two-book tradition. In figure 1, we create a program with `cargo new`, Rust's project generator — meet `cargo` now, because like `pip`, `venv`, `make`, and `pytest` fused into one tool, it will be everywhere.

```text
# Figure 1: Creating our first program

% cargo new hello
    Creating binary (application) `hello` package
```

`cargo new` writes a tiny project containing `src/main.rs`, which is where figure 2 lives. Where Python let us type `print("Hello, world.")` naked at a prompt, Rust asks for a function — `fn main()` is where every Rust program begins. The exclamation point on `println!` marks it as a *macro* rather than a function, a distinction that will matter a great deal in Chapter 21 and not at all before then.

```rust
# Figure 2: The classic first program

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

Notice what happened between `cargo run` and the greeting: a compile. Nothing about our two-line program was checked until we asked to run it — but *everything* about it was checked before it ran. Hold on to that trade; it is the whole book in miniature.

## The plan

Part I (Chapters 1–14) teaches the Rust you need, always against the Python you know: ownership where Python had garbage collection, traits where Python had inheritance, `Result` where Python had exceptions, `match` where Python had `if` chains and envy. Part II (Chapters 15–20) reaches the simulator: async/await — whose engine, you will be pleased to hear, works on the same principles as the cocotb scheduler you already understand — then triggers, signal handles, and testbenches 1.0 and 2.0. Part III (Chapter 21) is a single load-bearing chapter on macros, Rust's answer to decorators and metaclasses. Part IV (Chapters 22–39) rebuilds the UVM: components, the lifecycle, configuration, the factory's job (done, you may be surprised to learn, by closures), component communication, and sequences, ending at testbench 8.0 — the same summit as last time, by a steeper and more scenic route. Part V wraps the complete testbench into a reference template and looks ahead.

The TinyALU is waiting. It has not gotten any bigger, and this time, neither will our tolerance for runtime errors.

---

*End of Chapter 1 draft.*

