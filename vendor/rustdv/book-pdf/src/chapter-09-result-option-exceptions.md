# Chapter 9: Result, Option, and the End of Exceptions

> **In the UVM...** error handling depended on the dialect. Python reported failures with exceptions — a workplace metaphor explains them: hit an error, raise it to your boss, who raises it to their boss, up the chain until somebody handles it or it reaches the public as a crash with a traceback. We caught them with `try`/`except`/`finally`. SystemVerilog never had exceptions: a function that failed returned a sentinel value, logged a `uvm_error`, or `$fatal`ed the simulation outright.

Rust does not have exceptions. Not "discourages them," not "has them but calls them something else" — the mechanism does not exist. There is no `raise`, no `try`, no `except`, and nothing that silently unwinds through your function while it's minding its own business. SystemVerilog engineers may feel at home here — but hold the feeling, because SV's alternative was sentinel values nothing forced you to check and log messages nothing forced you to read. Rust's alternative has teeth.

And before Python readers mourn, remember what the boss metaphor was papering over. In Python, when you call a function, *nothing about that function tells you it might raise*. `nice_div()` looks exactly like a function that always returns a number. The fact that it can instead fling a `ZeroDivisionError` through your code is invisible — undocumented control flow that you discover at runtime, in our world usually forty minutes into a simulation.

Rust's answer is almost embarrassingly simple: **errors are ordinary values, and functions that can fail say so in their return type.** Two enums from the standard library do all the work:

- `Option<T>` — a value that might not be there.
- `Result<T, E>` — an operation that might fail, and if so, why.

You already have the tool for consuming both: `match`, from Chapter 4. This chapter is `match` earning its keep.

## Option: a value that might not be there

Python has one value for "nothing here": `None`. It shows up as a return value (`players.get(4)` returns `None` when number 4 isn't in the dictionary), as a default, and as a sentinel. And it carries a famous failure mode: `None` is a perfectly good Python object right up until you use it like the thing you expected, at which point you get an `AttributeError` — often far from the line that produced the `None`, which is what makes it such a satisfying bug to chase.¹ SystemVerilog's version is `null`: a handle that types like the real thing and detonates mid-simulation the first time anyone dereferences it.

Rust refuses to let "maybe nothing" hide inside an ordinary type. A `u8` is always a number; a `String` is always a string. When a value might be absent, the type says so:

```rust
enum Option<T> {
    Some(T),
    None,
}
```

This is just an enum with a payload, exactly like the ones you built in Chapter 7 — it happens to be defined in the standard library and generic over any type `T` (generics get their full treatment in Chapter 11; for now, read `Option<u8>` as "maybe a `u8`").

You met `Option` briefly in Chapter 8, because `HashMap` lookups return one. In Python, `players.get(4)` returns `None` for a missing key, and `players.get(4, "Not in database")` supplies a default. Figure 1 is the same lookup in Rust.

```rust
// Figure 1: A HashMap lookup returns Option

use std::collections::HashMap;

fn main() {
    let mut players = HashMap::new();
    players.insert(7, "Beckham");
    players.insert(10, "Messi");
    players.insert(11, "Salah");

    match players.get(&4) {
        Some(name) => println!("Number 4? {name}"),
        None => println!("Number 4? Not in database"),
    }

    let player = players.get(&4).copied().unwrap_or("Not in database");
    println!("Number 4? {player}");
}
```

```text
--
Number 4? Not in database
Number 4? Not in database
```

The two halves of figure 1 do the same job two ways. The `match` is the fundamental move: an `Option` is an enum, so we take it apart with `match`, and the compiler *requires* both arms. You cannot forget the `None` case — leaving it out is a compile error, not a 2 a.m. discovery. The second half uses `unwrap_or`, one of a family of convenience methods on `Option` that package up the common matches; it is the exact analog of passing `get()` a default in Python.

Here is the part worth slowing down for. In your old languages, the nothing-value from a failed lookup is the same shape as any other value — you can pass a `None` or a `null` along, store it in a transaction field, and only find out three function calls later. In Rust, an `Option<&str>` *is not* a `&str`. You cannot print it as a name, compare it to a name, or hand it to a function expecting a name. The compiler stops you at the line where you forgot to handle absence — which is to say, the far-from-the-bug failure mode — Python's `AttributeError`, SystemVerilog's null-handle crash — is not merely discouraged. It doesn't compile.

When you only care about one arm, `if let` from Chapter 4 reads better than a `match` with an empty arm:

```rust
if let Some(name) = players.get(&10) {
    println!("Found: {name}");
}
```

## Result: an operation that can fail

`Option` says "there might be nothing." `Result` says "this might fail, and here is why":

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

`T` is the type you get on success; `E` is the error type you get instead. Let's walk the classic failure scenarios, starting with the classic: dividing by zero.

```rust
// Figure 2: You still can't divide by zero

fn main() {
    let divisor: i32 = "0".parse().unwrap();
    println!("3/0 = {}", 3 / divisor);
}
```

```text
--
thread 'main' panicked at src/main.rs:3:26:
attempt to divide by zero
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

A word about the `parse`: it is there because it has to be. Write `let divisor = 0;` and rustc, able to *see* the zero, refuses to compile the division at all. Only a divisor the compiler cannot predict — one arriving at runtime, as bus values do — gets the chance to panic.

That is a **panic** — Rust's crash. We will come back to panics later in the chapter, because they have a specific job, and "report a divide by zero to the caller" is not it. When the possibility of failure is part of a function's honest contract, the function should return it. The standard library agrees: alongside the `/` operator, integers provide `checked_div`, which returns an `Option` — `None` instead of a crash.²

A Python `nice_div()` catches `ZeroDivisionError` and returns `math.inf`. Our integer version can't return infinity, but it can do something better: return a `Result` that names what went wrong. Figure 3 is `nice_div`, Rust edition.

```rust
// Figure 3: nice_div returns a Result instead of raising

#[derive(Debug)]
enum DivError {
    DivideByZero,
}

fn nice_div(dividend: u32, divisor: u32) -> Result<u32, DivError> {
    match dividend.checked_div(divisor) {
        Some(result) => Ok(result),
        None => Err(DivError::DivideByZero),
    }
}

fn main() {
    match nice_div(33, 2) {
        Ok(result) => println!("nice_div(33, 2) = {result}"),
        Err(err) => println!("You screwed up your division, human: {err:?}"),
    }
    match nice_div(3, 0) {
        Ok(result) => println!("nice_div(3, 0) = {result}"),
        Err(err) => println!("You screwed up your division, human: {err:?}"),
    }
}
```

```text
--
nice_div(33, 2) = 16
You screwed up your division, human: DivideByZero
```

Read the signature first: `fn nice_div(dividend: u32, divisor: u32) -> Result<u32, DivError>`. That return type is the whole philosophy in one line. In Python, `nice_div`'s ability to fail was a secret between the function body and whoever read the documentation; in SystemVerilog, it was a status flag you were free to ignore. In Rust it is in the signature, which means the *compiler* knows, which means every caller is forced to acknowledge it. The `match` in `main` is our `except` block — except it cannot be forgotten, because you cannot get the `u32` out of a `Result<u32, DivError>` without going through it.

Notice also who's who in the port: the `Err` arm of the `match` is playing the role of Python's `except ZeroDivisionError` block, and constructing `Err(DivError::DivideByZero)` is playing the role of `raise`. Same drama, but now the error travels as a return value, in plain sight.

### The exception that vanished

Python's next classic scenario is `nice_div(3, "zero")` — dividing an `int` by a `str`, which raises a `TypeError`, which requires a second `except` block to catch. Figure 4 ports that call to Rust.

```rust
// Figure 4: The TypeError scenario, ported to Rust

fn main() {
    match nice_div(3, "zero") {
        Ok(result) => println!("nice_div = {result}"),
        Err(err) => println!("Error: {err:?}"),
    }
}
```

```text
--
error[E0308]: mismatched types
 --> src/main.rs:4:24
  |
4 |     match nice_div(3, "zero") {
  |           -------- ^^^^^^ expected `u32`, found `&str`
  |           |
  |           arguments to this function are incorrect
```

It doesn't compile. Of the two failure categories Python needed `except` blocks for, one has simply left the building: passing the wrong *type* is not a runtime error to be caught, it is a program the compiler refuses to build. The uncaught `TypeError`, the second `except` block, catching two exception types on one line — none of it has a Rust equivalent, because the bug it handles cannot occur. Keep a tally of moments like this; the book has several more coming.³

## The `?` operator: propagation you can see

Back to the boss metaphor. The good idea inside exceptions was *delegation*: a low-level function shouldn't have to decide what a divide-by-zero means for the whole program. It should hand the problem upward. Python's `raise` did that invisibly. Rust does it with one visible character.

Suppose we're writing a little TinyALU-flavored utility: compute a ratio of two accumulated counts as a percentage. It calls `nice_div`, and if the division fails, our function can't succeed either — the error should go up to *our* caller. Written longhand, that's a `match` where the `Err` arm just re-returns the error. Written idiomatically, it's figure 5.

```rust
// Figure 5: The ? operator sends the error up the stack

fn percent(numerator: u32, denominator: u32) -> Result<u32, DivError> {
    let ratio = nice_div(numerator * 100, denominator)?;
    Ok(ratio)
}

fn main() {
    match percent(40, 0) {
        Ok(pct) => println!("{pct}%"),
        Err(err) => println!("percent failed: {err:?}"),
    }
}
```

```text
--
percent failed: DivideByZero
```

The `?` after `nice_div(...)` means: *if this is `Ok(value)`, unwrap it and keep going; if it is `Err(e)`, return `Err(e)` from this function, right now.* That is exception propagation — the error bubbles up the call stack, each function passing it to its boss — with two differences that change everything:

1. **It's visible at the call site.** Every fallible call in a Rust function is marked with a `?` (or an explicit `match`). Scanning a function body tells you exactly where it can bail out early. A Python function body gives you no such list; any line might throw.
2. **It's visible in the signature.** You can only use `?` in a function that itself returns a `Result` (the compiler enforces this, with a helpful error if you forget). So the ability to fail is contagious *in the types*: if `percent` propagates `nice_div`'s errors, then `percent`'s signature says `Result`, and its callers are on notice too. The chain of bosses is written down.

Python's re-raise pattern — catch an exception, print a snarky message, then `raise` it onward — becomes a `match` (or an `inspect_err` call) that logs and then returns the `Err`. Nothing new to learn, just values. And when the error types along the chain differ, `?` will convert between them automatically if you've told it how; that hook is a trait called `From`, and traits are the very next chapter, so we'll leave that thread hanging deliberately.

One more nicety: `main` itself can return a `Result`. When it returns an `Err`, the program prints the error and exits with a failing status — the last boss in the chain has a sensible default. In Part II you'll see that rustdv tests work the same way: a test is an `async fn` returning `Result<(), TestError>`, and an `Err` fails the test. The `?` operators sprinkled through a testbench are little arrows pointing at everything that can end the test.

## Designing an error enum

`DivError` had one variant, which made it a demo. Real error types earn their keep when there are several ways to fail and the caller might care which one happened — exactly the job Python's exception *class hierarchy* did, with `except ZeroDivisionError` and `except TypeError` selecting by type. In Rust, the error type is an enum, the variants are the failure modes, payloads carry the evidence, and `match` does the selecting.

Let's build one the TinyALU will actually need. Think ahead to a testbench utility that decodes an operation field from the DUT into our `Ops` enum from Chapter 7. Two things can go wrong: the bits might not encode any legal operation, or the DUT might never hand us the value at all before the clock runs out. That's a two-variant enum, one variant carrying the offending byte:

```rust
// Figure 6: A custom error enum for the TinyALU

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum AluError {
    InvalidOp(u8),
    Timeout,
}

impl fmt::Display for AluError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AluError::InvalidOp(bits) => {
                write!(f, "invalid ALU op code: {bits:#04x}")
            }
            AluError::Timeout => write!(f, "timed out waiting for the DUT"),
        }
    }
}

fn decode_op(bits: u8) -> Result<Ops, AluError> {
    match bits {
        1 => Ok(Ops::Add),
        2 => Ok(Ops::And),
        3 => Ok(Ops::Xor),
        4 => Ok(Ops::Mul),
        _ => Err(AluError::InvalidOp(bits)),
    }
}

fn main() {
    for bits in [1, 4, 7] {
        match decode_op(bits) {
            Ok(op) => println!("{bits} decodes to {op:?}"),
            Err(err) => println!("decode failed: {err}"),
        }
    }
}
```

```text
--
1 decodes to Add
4 decodes to Mul
decode failed: invalid ALU op code: 0x07
```

Walk through what each piece buys:

- **`#[derive(Debug, ...)]`** gives us the `{err:?}` developer-facing printout for free — the same derive habit from Chapters 7 and 8.
- **The `Display` implementation** is the human-facing message, the counterpart of the string you'd pass when raising a Python exception or logging a `uvm_error` (`raise ValueError(f"invalid op: {bits}")`). Writing `impl fmt::Display for AluError` is our first hand-written trait implementation; Chapter 10 will explain the machinery you just used. For now: implementing `Display` is what makes `{err}` (no `:?`) work, and it's the conventional courtesy every public error type pays.
- **The payload on `InvalidOp(u8)`** carries the evidence to wherever the error is handled. Python attached this data to the exception object; we attach it to the variant. No fishing it back out of a message string.
- **Callers select with `match`.** A caller who wants to retry on `Timeout` but fail hard on `InvalidOp` writes a two-arm match — the moral equivalent of two `except` blocks, checked for exhaustiveness by the compiler. Add a third variant to `AluError` next month, and every such `match` in the codebase becomes a compile error until it says what to do about the new case. Try getting *that* from an exception hierarchy.

This pattern — an enum of failure modes, `Debug` derived, `Display` implemented, payloads where useful — is the whole craft of error design in application code, and it's the shape rustdv's own errors take (`HandleError` for signal lookups, `TestError` for test outcomes — you'll meet them in Chapter 17).

## panic!: for bugs, not for failures

Now we can return to figure 2's crash. `panic!` is Rust's mechanism for *this program has a bug*: it prints a message and location, unwinds the current thread, and by default takes the program down. You can invoke it yourself:

```rust
panic!("driver state machine reached an impossible state");
```

The panicking family has members you will use daily:

- **`assert!(condition, "message...")`** — panic if the condition is false. `assert_eq!(a, b)` panics if two values differ, and prints both. SystemVerilog engineers know immediate assertions well; the difference is disposition — an SV assertion failure prints and, by default, the simulation soldiers on, while a Rust `assert!` stops the program at the scene.
- **`.unwrap()`** — on an `Option` or `Result`: give me the value, and panic if it's `None`/`Err`.
- **`.expect("message")`** — `unwrap` with a message you choose, which makes it strictly better in code you keep.⁴

Figure 7 puts `assert!` to work in a checksum helper, `xor_bytes`. Rust sharpens the classic example: `xor_bytes` takes `&[u8]`, so a value over 255 can't even reach the function (the `TypeError` disappearance, again). What still *can* go wrong is a claim about our own logic — say, this version that folds in a parity check the surrounding testbench relies on:

```rust
// Figure 7: assert! guards an invariant

fn xor_bytes(bytes: &[u8]) -> u8 {
    let mut xor = 0;
    for b in bytes {
        xor ^= b;
    }
    xor
}

fn main() {
    let frame = [0x08, 0x09, 0x10];
    let checksum = xor_bytes(&frame);
    println!("checksum: {checksum:#04x}");
    assert!(
        xor_bytes(&[checksum, 0x11]) == 0,
        "checksum self-test failed: {checksum:#04x} ^ 0x11 != 0"
    );
    assert!(
        xor_bytes(&[checksum, 0x12]) == 0,
        "checksum self-test failed: {checksum:#04x} ^ 0x12 != 0"
    );
    println!("all self-tests passed");
}
```

```text
--
checksum: 0x11
thread 'main' panicked at src/main.rs:16:5:
checksum self-test failed: 0x11 ^ 0x12 != 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

The first assertion passes silently; the second one stops the program at the line where the impossible happened, with our message. That is what you want from an invariant check: loud, early, and located.

Python has a famous `assert` trap: `assert (8 < 7, "Obviously false")` never fires, because the parentheses build a two-element tuple, and a non-empty tuple is truthy. I am pleased to report the Rust translation of that bug: `assert!((8 < 7, "Obviously false"))` does not compile, because a tuple is not a `bool`, and `assert!` insists on a `bool`. The trap is not merely avoided; it is unrepresentable.

So when do you panic and when do you return `Err`? The line is intent:

- **`Result::Err` is for failures the design expects.** A signal that might not exist, an operand that might be out of range, a check that might not pass. These are outcomes, and the caller gets to decide what they mean.
- **`panic!`/`assert!` are for states the design promises are impossible.** If one fires, the code — not the input, not the DUT — is wrong, and there is no sensible way to continue.

`.unwrap()` and `.expect()` sit exactly on this line, which is why they need judgment: each one converts an `Err`/`None` into a panic, so each one is a small signed statement that says "I claim this cannot fail here." In playground code and examples, unwrap freely. In a testbench you'll run for a year, prefer `expect` with a message that will make sense to whoever reads the panic — probably you, later, in a worse mood.

One Python comfort has no direct Rust twin: `finally`. Rust's guarantee of "this cleanup runs no matter what" doesn't live in error-handling syntax at all — it lives in ownership. When a scope ends, by `return`, by `?`, or by panic-unwinding, values are dropped and their destructors run, as you saw in Chapter 5. Cleanup-on-any-exit is not something you remember to write in Rust; it's where the `Drop` happens.

## The taxonomy this book will live by

Everything above compresses into one convention, and it is load-bearing: the rest of this book — and the design of rustdv itself — assumes it.

> **The failure taxonomy.**
> **`Result::Err` is for checks** — the DUT did something wrong. A scoreboard comparing predicted against actual and finding a mismatch produces an `Err`. The test fails, which is the test doing its job. This is *expected fallibility*: finding these is why we come to work.
> **`panic!`/`assert!` are for testbench bugs** — *we* did something wrong. A driver calling `item_done` twice, a queue that is empty when the protocol guarantees it can't be, a state machine in a state the match arms say is impossible. The testbench is broken, and no result it reports can be trusted until it's fixed.

Both fail the test — rustdv catches panics at the task boundary and scores them as failures, just as cocotb caught a stray exception in any task — but the report distinguishes them, because the reader of the report must react differently: an `Err` sends you to the waveform viewer; a panic sends you to your own source code. It is the difference between the lab reporting that the chip failed and the lab reporting that the thermometer is broken.

Both of your languages blurred this line. In Python, an `AssertionError` from a scoreboard check and an `AttributeError` from a testbench typo were just exceptions rising through the same machinery, distinguished by reading the traceback. In SystemVerilog, `uvm_error` for DUT misbehavior versus `uvm_fatal` for testbench disasters was the right convention — but it was only a convention, nothing enforced it, and the simulation ran on either way until somebody read the log. Rust gives the two categories different mechanisms, different types, and different syntax — so from Chapter 18 on, when you see a scoreboard whose check returns `Result` and a driver studded with `assert!`, you are seeing this chapter's taxonomy at work, not a stylistic accident.

You now hold both halves of Rust's honesty policy: types that admit absence (`Option`), and signatures that admit failure (`Result`). Along the way, you implemented your first trait — that `Display` on `AluError` — mostly on faith. Chapter 10 replaces the faith with understanding: traits are how Rust does everything Python did with inheritance and dunder methods, and they are the last big idea between you and real testbench components.

> ¹ In Python, `players.get(4)` returns `None` for a missing player. The bug arrives when you then write `player.upper()` — and Python tells you `'NoneType' object has no attribute 'upper'`, three files away from the lookup.
>
> ² Python's `nice_div` returned `math.inf` for division by zero, on the theory that it's the mathematically correct answer. Rust's *floating-point* division agrees — `3.0 / 0.0` is `inf`, per IEEE 754, no panic — so on this one point the languages are in complete accord and only the integers object.
>
> ³ Chapter 27 is essentially this moment stretched to a full chapter: every config-database failure mode the UVM ever taught you to debug, replayed as a compile error.
>
> ⁴ A colleague of mine calls `.unwrap()` "a panic with no commit message."
