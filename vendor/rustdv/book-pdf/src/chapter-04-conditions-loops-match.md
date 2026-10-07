# Chapter 4: Conditions, Loops, and Match

Conditions and loops are where your old languages already agree: Python tests with `if`/`elif`/`else` and loops with `while` and `for`; SystemVerilog tests with `if`/`else` and `case` and loops with everything from `for` to `forever`. Two-thirds of this chapter is warm-up, because these constructs transfer to Rust almost without friction. The last third introduces the construct the rest of this book leans on constantly: `match` — the statement Python never had and the one SystemVerilog's `case` always wanted to be. When we meet `Result` in Chapter 9, `Option` alongside it, and the `Ops` enum in Chapter 7, `match` is how we will take them apart. Learn it well here, in miniature, and every later chapter gets easier.

## if and else

Rust's `if` looks like Python's with the punctuation swapped: braces instead of indentation, and no colon. Like Python — and unlike C and SystemVerilog — Rust does not require parentheses around the condition. Unlike everybody, Rust *insists* on the braces, even for one-line bodies.

```rust
// Figure 1: A Rust if statement

fn main() {
    let name = "Roy";
    if name != "Danny" {
        println!("Hey, you're not Danny.");
    }
}
```

```text
--
Hey, you're not Danny.
```

One difference matters more than it looks. Python happily tested any value for truth: `if nn:` meant "if `nn` is nonzero," `if my_list:` meant "if the list is nonempty." Rust conditions must be `bool` — actually, literally `bool`. Write `if nn {` where `nn` is an integer and the compiler stops you:

```rust
// Figure 2: Rust has no truthiness

fn main() {
    let nn = 5;
    if nn {
        println!("nonzero");
    }
}
```

```text
--
error[E0308]: mismatched types
 --> src/main.rs:3:8
  |
3 |     if nn {
  |        ^^ expected `bool`, found integer
```

You will grumble about this for a week and then remember every testbench where `if dut.done:` silently tested the wrong thing — a handle instead of a value, a list instead of its contents. Rust makes you write `if nn != 0`, and the intent goes on the record.¹

There is no `elif` keyword. Rust spells it `else if`, and a chain of them stands in for a switch — for now:

```rust
// Figure 3: else if as a switch (for now)

fn main() {
    let (a, b) = (5, 5);
    let operation = "divide";
    if operation == "add" {
        println!("A + B = {}", a + b);
    } else if operation == "subtract" {
        println!("A - B = {}", a - b);
    } else if operation == "multiply" {
        println!("A * B = {}", a * b);
    } else {
        println!("Illegal Operation: {operation}");
    }
}
```

```text
--
Illegal Operation: divide
```

Python called this trade "more flexibility at the cost of more code," and SystemVerilog readers reaching for `case` should hold that reflex a few pages. The cost is about to be refunded, with interest, in the `match` section.

> ¹ SystemVerilog veterans have the opposite scar: `if (sig)` on a 4-state signal, where an `X` quietly takes the false branch. Rust's answer to both languages is the same: say what you mean, and the compiler will hold you to it.

## if is an expression

Here is the first new idea. In Chapter 3 we met Rust's distinction between statements and expressions: an expression produces a value. In Rust, `if`/`else` *is an expression* — the whole construct evaluates to the value of whichever branch ran. That means you can bind it with `let`:

```rust
// Figure 4: Conditional assignment — no ternary needed

fn main() {
    let aa = 5;
    let message = if aa == 5 { "five_val" } else { "other_val" };
    println!("{message}");
}
```

```text
--
five_val
```

Python needed special ternary syntax — `x if cond else y` — because its `if` statement produces nothing. Rust needs no ternary because its ordinary `if` already produces a value.² Two rules come along with this power. First, both branches must produce the *same type* — a branch handing back a string while the other hands back an integer is a compile error, because `message` must be exactly one type. Second, if you use `if` as an expression, the `else` is mandatory; the compiler will not let you bind a value that might not exist. Both rules are the compiler asking the question Python deferred to runtime: *what, exactly, is this variable?*

> ² C and SystemVerilog programmers may now retire the `?` operator with full honors.

## Three loops, one of them new

Python has two loops, `while` and `for`. SystemVerilog, characteristically, has five. Rust has three: `while`, `for`, and `loop`. The first two are your old friends in new clothes; the third will look familiar to exactly half of you.

`while` works exactly as you expect, condition first, braces around the body. Here is a counting loop:

```rust
// Figure 5: A while loop in action

fn main() {
    let mut nn = 0;
    while nn <= 13 {
        print!("{nn} ");
        nn += 1;
    }
    println!();
}
```

```text
--
0 1 2 3 4 5 6 7 8 9 10 11 12 13
```

Two Chapter 3 details show up here: `nn` needs `mut` because we reassign it, and `print!` (without the `ln`) is how Rust says `end=" "` — it prints without the newline. `continue` and `break` exist, spelled the same and meaning the same as in Python; I will not re-teach them.

Now the third loop. Python emulates the run-forever pattern with `while True:` and a well-placed `break`; SystemVerilog gave it a keyword, `forever`. Rust agrees with SystemVerilog — intentionally infinite loops are common enough to deserve their own construct — and then improves the idea:

```rust
// Figure 6: loop — the intentional infinite loop

fn main() {
    let mut nn = 0;
    let first_big_square = loop {
        nn += 1;
        if nn * nn > 200 {
            break nn * nn;
        }
    };
    println!("{first_big_square}");
}
```

```text
--
225
```

Look closely at that `break`: it carries a value, and the whole `loop` evaluates to it — which is why we could write `let first_big_square = loop { ... }`. Like `if`, `loop` is an expression. A "run until something happens, then hand back what you found" pattern that took a flag variable and a post-loop read in Python is one construct in Rust. Only `loop` gets this privilege; `while` and `for` loops cannot `break` with a value, because their conditions mean they might never produce one.

If `loop` sounds like a novelty, consider what you already know is coming: every driver loop and monitor loop you have ever written was a `forever` or a `while True:` at heart. In Part II, those become `loop` — the language admitting what the code always meant.

## Ranges

Python's `range()` was a constructor with three calling conventions. Rust builds ranges from operators instead:

- `0..8` — start at 0, stop *before* 8. The same half-open interval as `range(0, 8)`.
- `0..=8` — start at 0, stop *at* 8, inclusive. Python had no direct equivalent; you wrote `range(0, 9)` and remembered why.

The `for` loop iterates over a range just as Python's did:

```rust
// Figure 7: Looping through numbers using a range

fn main() {
    for ii in 0..4 {
        print!("{ii} ");
    }
    println!();
}
```

```text
--
0 1 2 3
```

Where is `step`? Ranges are iterators (Chapter 12 makes that concept rigorous), and iterators have adapter methods. Python's `range(1, 14, 2)` becomes:

```rust
// Figure 8: Stepping through a range

fn main() {
    for ii in (1..14).step_by(2) {
        print!("{ii} ");
    }
    println!();
}
```

```text
--
1 3 5 7 9 11 13
```

There is also `.rev()` for counting down, and a whole catalog of adapters waiting in Chapter 12. For now: `..` exclusive, `..=` inclusive, adapters for everything else. And notice `0..=8` reads naturally for hardware — an inclusive range is how you think about the values a bus can carry. A TinyALU operand is a `u8`; the values it can take are `0..=255`, and you can write exactly that.

## match: the construct Python never had

Now the centerpiece. Python replaced `case`/`switch` with `elif` chains and called the trade "more flexibility at the cost of more code." Rust's `match` refuses the trade: it delivers more flexibility than `case` *and* less code than `elif` — and then adds a property neither language offered, one that this book will spend many chapters collecting dividends on.

Start with the direct port. Figure 3's `else if` chain becomes:

```rust
// Figure 9: match as a switch

fn main() {
    let (a, b) = (5, 5);
    let operation = "multiply";
    let answer = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        _ => panic!("Illegal Operation: {operation}"),
    };
    println!("answer = {answer}");
}
```

```text
--
answer = 25
```

Read it as: compare `operation` against each *pattern* on the left of a `=>`; run the code on the right of the first pattern that fits. The underscore `_` is the wildcard — it matches anything, playing the role of `else` or `default`. Three things to notice, each an upgrade over both `elif` and `case`:

1. **`match` is an expression.** Like `if` and `loop`, the whole construct produces a value — here it computes `answer` directly, where figure 3 could only print from inside each branch. Every arm must produce the same type, same rule as `if`.
2. **There is no fallthrough.** C and SystemVerilog programmers carry decades of missing-`break` scar tissue; `match` arms are separate, always, no `break` required or even possible.
3. **The compiler checks that the patterns cover every case.** This one gets its own section.

## Exhaustiveness: the compiler counts your cases

Delete the `_` arm from a `match` and something remarkable happens. Here is a `match` on a raw op code — the TinyALU's four operations, numbered 1 through 4 as the spec has always numbered them — with no wildcard:

```rust
// Figure 10: The compiler catches missing cases

fn main() {
    let op_code: u8 = 2;
    let name = match op_code {
        1 => "ADD",
        2 => "AND",
        3 => "XOR",
        4 => "MUL",
    };
    println!("{name}");
}
```

```text
--
error[E0004]: non-exhaustive patterns: `0_u8` and `5_u8..=u8::MAX` not covered
 --> src/main.rs:3:22
  |
3 |     let name = match op_code {
  |                      ^^^^^^^ not covered
  |
  = note: the matched value is of type `u8`
```

Sit with that error message for a moment, because it is doing something neither of your testbench languages ever did for free. The compiler enumerated every value a `u8` can hold, subtracted the four we handled, and reported precisely what we missed: zero, and everything from 5 up. An `elif` chain that forgets a case is a runtime surprise — figure 3 only caught its illegal `"divide"` because we remembered to write the `else`. A SystemVerilog `case` that forgets one falls through in silence unless you wrote the `default`, and even `unique case` merely upgrades the silence to a runtime warning that waits for the right stimulus to arrive before it speaks. A `match` that forgets a case *does not compile*. The fix is either a `_` arm (an explicit decision to lump the leftovers together) or arms for the missing values (an explicit decision about each) — but it is always a decision, never an oversight.

For a `u8`, exhaustiveness is a nice safety net. The reason this book teaches `match` in Chapter 4 rather than Chapter 14 is what happens when the thing being matched has a *small, meaningful* set of cases. In Chapter 7, `Ops` returns as a true Rust enum with exactly four values, and a `match` on it needs exactly four arms — no wildcard, no dead cases, and if the TinyALU ever grows a fifth operation, **every `match` in the testbench that fails to handle it becomes a compile error**. Your scoreboard, your coverage collector, your predictor: the compiler hands you the complete list of code that must learn about the new op.

## Patterns: ranges, tuples, and taking things apart

The left side of a `match` arm is not limited to constants. Patterns can be ranges — and here the TinyALU gives us a real example: ADD, AND, and XOR complete in one cycle while MUL takes three. With op codes 1 through 4:

```rust
// Figure 11: Matching on ranges

fn main() {
    let op_code: u8 = 4;
    let cycles = match op_code {
        1..=3 => 1,
        4 => 3,
        _ => panic!("Illegal op code: {op_code}"),
    };
    println!("This operation takes {cycles} cycle(s)");
}
```

```text
--
This operation takes 3 cycle(s)
```

Range patterns use the inclusive `..=` form, and you can also combine alternatives with `|`, as in `1 | 3 => ...`. The compiler still does its exhaustiveness arithmetic across all of it.

Patterns can also take structured data apart. Match on a tuple, and each position of the pattern matches the corresponding element — with `_` skipping positions you don't care about and plain names *binding* the values so the arm can use them:

```rust
// Figure 12: Matching and destructuring a tuple

fn main() {
    let operands: (u8, u8) = (0, 200);
    let comment = match operands {
        (0, 0) => String::from("both operands zero"),
        (0, _) | (_, 0) => String::from("one operand zero"),
        (a, b) if a == b => format!("equal operands: {a}"),
        (a, b) => format!("ordinary operands: {a}, {b}"),
    };
    println!("{comment}");
}
```

```text
--
one operand zero
```

Three new tricks in one figure. The `|` combines two patterns into one arm. The names `a` and `b` in the later arms are bindings — the arm receives the matched values under those names, which is how `match` goes beyond comparing and starts *extracting*. And `if a == b` is a *match guard*, an extra boolean test bolted onto a pattern for the cases patterns alone can't express. You have just watched a `match` classify stimulus into coverage-bin-shaped categories in four lines — remember this figure when we build the coverage collector.

That extraction ability is the real reason `match` anchors this book. Here is a preview of what is coming — do not worry about the details yet. In Chapter 9 you will learn that a fallible operation like looking up a DUT signal returns a `Result`, which is either `Ok(handle)` carrying the goods or `Err(e)` carrying the explanation. The way you get the goods out is a `match`:

```rust
match dut.child("clk") {
    Ok(clk) => { /* use clk */ }
    Err(e) => { /* the signal wasn't there; e says why */ }
}
```

One construct checks which case you got *and* hands you its contents *and* forces you — at compile time — to say what happens in the failure case you would rather not think about. Python's `try`/`except` let you skip the `except` and hope; SystemVerilog mostly declined to have an error story at all; `match` on a `Result` has no such loophole. Exhaustiveness, it turns out, is not a switch-statement garnish. It is how Rust makes error handling mandatory, and Chapters 7 and 9 are where that bill comes due — in our favor.

## Summary

Rust's conditions and loops hold no terrors: `if`/`else if`/`else` with mandatory braces and honest `bool` conditions, `while` and `for` behaving as you expect, `continue` and `break` unchanged. The differences all push the same direction. No truthiness, and no silently-tested `X`. `if` is an expression, retiring the ternary. `loop` is `forever` with a diploma — it names the intentional infinite loop and can `break` with a value. Ranges are syntax (`0..8` exclusive, `0..=8` inclusive) rather than a constructor, with iterator adapters like `step_by` covering the rest.

And `match` is the construct Python never had and `case` wanted to be: patterns instead of comparisons, expression instead of statement, no fallthrough, destructuring with bindings and guards — and exhaustiveness checking, the compiler's guarantee that every case is a decision and no case is an oversight. We will `match` on integers and tuples this week, on `Ops` in Chapter 7, and on `Option` and `Result` for the rest of our verification careers.

So far, every value we have used has lived and died inside `fn main` without our attention — garbage-collected habits, still serving us fine. In the next chapter, we hand a value from one variable to another and discover that Rust has been keeping track of who owns what all along. Chapter 5 is ownership: the idea with no mirror in either of your languages, and the hinge of the whole one you're learning.
