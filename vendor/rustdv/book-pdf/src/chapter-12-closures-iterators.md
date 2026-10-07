# Chapter 12: Closures and Iterators

Python made testbench code shorter and stranger at the same time with two features: comprehensions, which built a whole list in one bracketed line, and generators, which used `yield` to produce values one at a time without ever building the list at all. Both were ways of saying *here is a stream of values and a recipe for making them*. SystemVerilog never had either — nor the feature underneath them — which makes this chapter the newest ground in Part I for half of this book's readers, and worth every minute of it.

Rust has both ideas, reorganized around one feature: the **iterator**. And driving the iterator machinery is a smaller feature that this book has been saving up for eleven chapters, because it is quietly one of the most important in the language: the **closure**. Closures matter far beyond this chapter. When Chapter 29 rebuilds the UVM factory, the values its registry stores — the makers that construct components on demand — will turn out to be exactly this chapter's closures. This is the chapter where you learn why that sentence makes sense.

> **In the UVM...** stimulus recipes were loops. Python's dialect could also write them as generators — `yield` hands a value to the caller and *keeps running* — and as comprehensions like `[nn**2 for nn in range(11) if nn % 2 == 0]`, four parts in square brackets replacing a four-line loop. SystemVerilog's closest analog was a task feeding a mailbox: the stream-of-values idea was there, but functions were never values you could pass around, store, or build streams from.

## Closures: functions as values

A closure is a function without a name, written inline, that can use the variables around it. Python had these in two flavors: `lambda x: x + 1` for one-liners, and nested `def` for anything longer. SystemVerilog had nothing of the kind — an SV function has a name, a declaration, and an address in a package somewhere, and it certainly cannot be stored in a variable. Rust has one syntax, and the whole feature fits in a figure. The parameters go between vertical bars, and the body follows.

```rust
// Figure 1: A closure is an unnamed function in a variable

fn main() {
    let add_one = |x: u32| x + 1;

    let describe = |aa: u8, bb: u8| {
        let sum = aa as u16 + bb as u16;
        format!("{aa} + {bb} = {sum}")
    };

    println!("{}", add_one(41));
    println!("{}", describe(0xFF, 0x01));
}
```

```text
--
42
255 + 1 = 256
```

`add_one` holds a function the way a variable holds a number. A single expression needs no braces; a multi-line body takes braces and, like every Rust block, evaluates to its last expression. Notice there is no return type written on either closure — the compiler infers closure types from how you use them, which is why closures usually look lighter than `fn` declarations. In fact the parameter types are usually optional too; I wrote `x: u32` for clarity, but `let add_one = |x| x + 1;` compiles fine once the compiler sees a call that pins the type down.

So far this is `lambda` with different punctuation. The interesting part — the part Python never asked you to think about — is what happens when the closure uses a variable it did not declare.

## Capture, through the ownership lens

A Python closure that mentions an outer variable just... uses it. Every Python name is a reference, so the closure captures a reference, silently, and if two pieces of code mutate the same captured object at the same time, that is your problem to discover at runtime. Chapter 6's shared-variable race — two processes sharing `transaction_data` — was exactly this bug wearing a coroutine costume.

Rust closures also capture outer variables, but here is the difference: *capturing is subject to the ownership rules from Chapters 5 and 6, like everything else.* A closure that reads a variable borrows it with `&T`. A closure that mutates one borrows it with `&mut T`. A closure that consumes one takes ownership. The compiler looks at the closure's body, picks the least drastic mode that works, and then enforces it — visibly, in the type system, at compile time.

Watch the three modes in order. First, a closure that only reads.

```rust
// Figure 2: A closure that reads captures by shared borrow

fn main() {
    let ops = vec!["ADD", "AND", "XOR", "MUL"];

    let show = || println!("ops under test: {ops:?}");

    show();
    show();
    println!("still mine: {} ops", ops.len());  // ops was only borrowed
}
```

```text
--
ops under test: ["ADD", "AND", "XOR", "MUL"]
ops under test: ["ADD", "AND", "XOR", "MUL"]
still mine: 4 ops
```

`show` borrowed `ops` the way any `&Vec` would, so we can call it repeatedly and still use `ops` afterward. Second, a closure that mutates.

```rust
// Figure 3: A closure that mutates captures by exclusive borrow

fn main() {
    let mut errors = Vec::new();

    let mut log_error = |msg: &str| errors.push(msg.to_string());

    log_error("ADD result mismatch");
    log_error("XOR result mismatch");

    println!("{} errors: {errors:?}", errors.len());
}
```

```text
--
2 errors: ["ADD result mismatch", "XOR result mismatch"]
```

Two things changed. The closure itself must be declared `let mut`, because calling it mutates the captured `errors` — mutation is never invisible in Rust, not even here. And while `log_error` is alive, it holds the *exclusive* borrow of `errors`; if we tried to `println!("{errors:?}")` between the two calls, the compiler would refuse, citing the aliasing-XOR-mutability rule from Chapter 6. One writer, no readers alongside. The race your old languages could only warn about is structurally impossible to write.

Third, a closure that takes ownership. Sometimes a closure must own its captures — most often because it will outlive the scope it was created in, which is precisely the situation when you store a closure in a struct or hand it to another task. The `move` keyword forces the transfer.

```rust
// Figure 4: A move closure takes ownership of its captures

fn main() {
    let test_name = String::from("alu_smoke_test");

    let banner = move || format!("*** {test_name} ***");

    println!("{}", banner());
    println!("{}", test_name);  // ERROR: test_name moved into the closure
}
```

```text
--
error[E0382]: borrow of moved value: `test_name`
 --> src/main.rs:8:20
  |
4 |     let banner = move || format!("*** {test_name} ***");
  |                  ------- value moved into closure here
...
8 |     println!("{}", test_name);
  |                    ^^^^^^^^^ value borrowed here after move
```

The error message tells the whole story: `test_name` moved into `banner`, and Chapter 5's rule applies — after a move, the old name is dead. Delete the last `println!` and the program compiles. This is the same monitor-hands-transaction-to-scoreboard reasoning you already know; the only novelty is that the new owner is a closure instead of a function parameter.

Rust names these three capture behaviors with three traits, and you will meet them constantly in documentation: **`Fn`** for closures that can be called any number of times through a shared borrow (figure 2), **`FnMut`** for closures that mutate and need exclusive access to call (figure 3), and **`FnOnce`** for closures that consume something and therefore can only be called once.¹ You do not choose among them by annotation — the compiler classifies each closure from its body — but you will *read* them in every function signature that accepts a closure, and later in this chapter you will write one into a struct field.

> ¹ The names describe how the closure may be *called*, and they nest: every `Fn` is also an `FnMut`, and every `FnMut` is also an `FnOnce` — a closure you may call many times can certainly be called once. Interviewers love this; day-to-day code mostly just needs you to recognize the three names.

## Iterator adapters: comprehensions, unrolled

Now the payoff. Python builds `even_squares` in one famous line:

```python
even_squares = [nn**2 for nn in range(11) if nn % 2 == 0]
```

A comprehension has four parts: an expression, a variable, an iterator, and a filter. Rust has no comprehension syntax. Instead it lets you take those same four parts and chain them left to right as method calls on the iterator — each method taking, naturally, a closure.

```rust
// Figure 5: The list comprehension, as an iterator chain

fn main() {
    let even_squares: Vec<u32> = (0..=10)
        .filter(|nn| nn % 2 == 0)
        .map(|nn| nn * nn)
        .collect();

    println!("even squares {even_squares:?}");
}
```

```text
--
even squares [0, 4, 16, 36, 64, 100]
```

Read the chain aloud and it is the comprehension in sentence order: take the range zero through ten, *filter* it down to the even numbers, *map* each survivor to its square, and *collect* the results into a `Vec`. The `|nn| ...` closures are the comprehension's expression and filter parts, now explicit values passed as arguments. Where Python's comprehension makes you learn the four positions inside the brackets, the Rust version wears its structure on the outside — and when a chain grows too clever, it splits across lines exactly as figure 5 shows, no special multi-line dispensation required.

Python's dictionary comprehension ports the same way. `{ii : ii**3 for ii in range(4)}` becomes a chain that maps each number to a `(key, value)` pair and collects into a `HashMap`:

```rust
// Figure 6: The dictionary comprehension, collected into a HashMap

use std::collections::HashMap;

fn main() {
    let cubes: HashMap<u32, u32> = (0..4)
        .map(|ii: u32| (ii, ii.pow(3)))
        .collect();

    println!("cubes: {cubes:?}");
}
```

```text
--
cubes: {2: 8, 0: 0, 3: 27, 1: 1}
```

Two things to notice. `collect()` is doing something quietly remarkable: the *same method* built a `Vec` in figure 5 and a `HashMap` here, steered by the type annotation on the left — the generics machinery from Chapter 11 earning its keep. (One wrinkle: the closure's parameter carries its own `: u32`, because a method call like `.pow` must know its receiver's concrete type on the spot — inference has not yet flowed backward from the annotation when the closure body is checked.) And look at that output order. Chapter 8 warned you that Rust's `HashMap`, unlike a Python dict or an SV associative array, promises nothing about iteration order, and here is the proof; your run will likely print a different scramble.

One more adapter completes the everyday set. Where `map` transforms each element and `filter` drops some, **`fold`** boils the whole stream down to a single value: it takes a starting accumulator and a closure that combines the accumulator with each element in turn. Here is a scoreboard-flavored example — counting mismatches in a list of (expected, actual) pairs:

```rust
// Figure 7: fold reduces a stream to one value

fn main() {
    let results = [(0x55u16, 0x55u16), (0x100, 0x100), (0x0FE, 0x0FF)];

    let mismatches = results
        .iter()
        .fold(0, |errs, (exp, act)| if exp == act { errs } else { errs + 1 });

    println!("mismatches: {mismatches}");
}
```

```text
--
mismatches: 1
```

In truth you will reach for `fold` less often than you expect, because the standard library pre-packages the common folds: `.sum()`, `.count()`, `.max()`, `.any()`, `.all()`. The chain `results.iter().filter(|(exp, act)| exp != act).count()` does figure 7's job and reads better. But `fold` is the general case the shortcuts are made of, and knowing it makes the shortcuts unmysterious.

## Lazy, like a generator

Here is the fact that connects comprehensions to generators, and it deserves its own paragraph: **iterator adapters do nothing until something consumes them.** The chain `(0..=10).filter(...).map(...)` computes no squares. It builds a small struct that *describes* the computation, and only when `collect()` — or a `for` loop, or `sum()` — starts pulling values through does any work happen, one element at a time, no intermediate list anywhere.

If that sounds familiar, it should. It is exactly the property that made Python generators worth a chapter: `my_range(1_000_000)` didn't build a million-entry list, it produced values on demand. In Rust, *every* iterator chain behaves that way by default. Python made laziness an opt-in feature with special syntax; Rust made it the only behavior and never needed the syntax.

Which raises the obvious question: what happened to `yield`?

## Where generators went

Rust has no `yield` statement.² What it has instead is the `Iterator` trait — one required method, `next()`, which returns `Some(value)` until the stream is exhausted and `None` thereafter. That `Option` should ring a bell from Chapter 9: where Python generators signal exhaustion with a `StopIteration` exception behind the scenes, Rust signals it in the return type, in the open.

Any type that implements `Iterator` works in a `for` loop, chains with every adapter above, and collects into collections. The classic generator is Fibonacci, so let us port it. Where a Python generator kept `lastnumb` and `numb` alive between `yield`s inside a paused function, Rust keeps them as fields in a struct, and `next()` advances the state one step per call.

```rust
// Figure 8: The Fibonacci generator, as an Iterator implementation

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
```

```text
--
0 1 1 2 3 5 8 13
```

The state that Python hid in a suspended stack frame is now a two-field struct you can see, and the resumption that Python performed by magic is now an ordinary method call. Notice that this iterator never returns `None` — it is *infinite*, which is fine, because it is lazy; `.take(8)` is an adapter that cuts the stream off after eight values. An infinite generator was a slightly daring trick in Python. In Rust it is Tuesday.

Writing an `impl Iterator` block for every one-off stream would get old, though, and here Chapter 11's `impl Trait` makes its second appearance — this time in a return position. A function can declare that it returns *some* iterator without naming the struct, and inside, you build that iterator however is clearest — with an ordinary loop, or from ranges and adapters. This is the closest Rust idiom to "a function with `yield` in it," and it is how this book will write value streams from here on.

Suppose a test wants every combination of small A and B operands for the TinyALU. In Python:

```python
def operand_pairs(n):
    for aa in range(n):
        for bb in range(n):
            yield (aa, bb)
```

And in Rust, as a function returning `impl Iterator`. The honest first translation keeps the Python's two loops exactly, and changes only one thing:

```rust
// Figure 9: A TinyALU operand-pair stream, replacing a generator function

fn operand_pairs(n: u8) -> impl Iterator<Item = (u8, u8)> {
    let mut pairs = Vec::new();
    for aa in 0..n {
        for bb in 0..n {
            pairs.push((aa, bb));
        }
    }
    pairs.into_iter()
}

fn main() {
    for (aa, bb) in operand_pairs(3) {
        print!("({aa},{bb}) ");
    }
    println!();
}
```

```text
--
(0,0) (0,1) (0,2) (1,0) (1,1) (1,2) (2,0) (2,1) (2,2)
```

Line for line, the body *is* the Python: the same two loops, in the same order. Exactly one thing changed, at the end. Python's `yield` handed each pair back the instant it was made; stable Rust has no `yield`, so instead you fill a `Vec` and hand back its iterator. `pairs.into_iter()` turns the vector into a stream of the same `(u8, u8)` items the signature promised, and the caller's `for` loop cannot tell it apart from a generator. That is the whole lesson of this figure: a function that returns `impl Iterator` is how Rust writes "a function that produces a stream of values."

There is one price, and paying it is the next figure. This version builds the entire vector before it returns a single pair, where Python's generator produced each pair on demand. For nine operand pairs that costs nothing — but the lazy form is worth seeing on its own, both because it is what you will meet in other people's code and because it is where the two `move` keywords from figure 4 stop being a curiosity and start earning their keep:

```rust
// Figure 10: The operand-pair stream, built lazily from adapters

fn operand_pairs(n: u8) -> impl Iterator<Item = (u8, u8)> {
    (0..n).flat_map(move |aa| (0..n).map(move |bb| (aa, bb)))
}

fn main() {
    for (aa, bb) in operand_pairs(3) {
        print!("({aa},{bb}) ");
    }
    println!();
}
```

```text
--
(0,0) (0,1) (0,2) (1,0) (1,1) (1,2) (2,0) (2,1) (2,2)
```

The one new idea here is `flat_map`, the nested-loop adapter: for each `aa` it runs the inner closure, which produces a whole stream of `(aa, bb)` pairs, and `flat_map` splices those inner streams end to end — the outer `for aa` and the inner `for bb`, rewritten as adapters. Same nine pairs, same order, but now nothing is computed until the caller asks for the next one. And *that* laziness is what forces the two `move`s. The inner closure must own its copy of `aa`, and the outer must own `n`, because these closures ride out of the function inside the returned iterator and are called long after `operand_pairs`'s own variables are gone. The capture rules you learned through the ownership lens are exactly what make it safe to return a paused computation from a function. Python kept the whole stack frame alive on the heap to manage this; Rust moves in precisely the values the closures need, and the compiler names each one if you forget the `move`.

> ² Generator syntax has been experimented with in unstable Rust for years, but stable Rust — the Rust this book teaches — does not have it, and between adapters and `impl Iterator`, you will rarely feel the gap.

## Closures you keep: a seed for Chapter 29

Everything so far has passed closures *downward* — into `map`, into `filter`, used and forgotten. The last idea in this chapter is the one with the longest reach in this book: a closure is a value, and like any value, it can be **stored in a struct field** and called later, by code that has no idea what the closure does inside.

Because every closure has its own unwritable type, storing one takes a trait object — Chapter 10's `dyn`, boxed up: `Box<dyn Fn(u8, u8) -> u16>` is "some heap-allocated callable, taking two `u8`s, returning a `u16`; I don't know or care which one." Here is a toy checker whose *prediction function is data*:

```rust
// Figure 11: A struct that carries its behavior as a closure

struct Checker {
    predict: Box<dyn Fn(u8, u8) -> u16>,
}

impl Checker {
    fn check(&self, aa: u8, bb: u8, actual: u16) {
        let expected = (self.predict)(aa, bb);
        if expected == actual {
            println!("PASS: ({aa}, {bb}) -> {actual}");
        } else {
            println!("FAIL: ({aa}, {bb}) expected {expected}, got {actual}");
        }
    }
}

fn main() {
    let adder_check = Checker {
        predict: Box::new(|aa, bb| aa as u16 + bb as u16),
    };
    adder_check.check(0xFF, 0x01, 0x100);

    let and_check = Checker {
        predict: Box::new(|aa, bb| (aa & bb) as u16),
    };
    and_check.check(0x0F, 0x35, 0x0006);  // wrong on purpose
}
```

```text
--
PASS: (255, 1) -> 256
FAIL: (15, 53) expected 5, got 6
```

Two `Checker` values, one struct definition, two completely different behaviors — selected not by inheritance, not by overriding a virtual method, but by *which closure was placed in the field at construction time*. Sit with that for a moment, because it is the seed of something large. The UVM factory — the registry, the `type_id::create()` calls, the override tables — exists to answer one question: *how does a test change what the testbench builds without editing the testbench?* rustdv keeps the registry, because create-by-name and overrides installed at a distance need one — and what that registry *stores*, one per component type, is a maker: a constructor as a value, a closure in a box, exactly like `predict` here. Where SystemVerilog manufactures its makers with `type_id` proxy classes and pyuvm with a metaclass, Rust just writes the closure down. You now hold the mechanism the registry stores; Chapter 29 supplies the methodology around it.

## Summary

Closures are unnamed functions in variables: `|x| x + 1`, with braces for multi-line bodies and types mostly inferred. They capture surrounding variables under the ordinary ownership rules — shared borrow to read, exclusive borrow to mutate, ownership when `move`d — and the traits `Fn`, `FnMut`, and `FnOnce` name those three calling contracts in signatures. Iterator adapter chains — `filter`, `map`, `flat_map`, `fold`, `collect` — replace Python's list, set, and dictionary comprehensions part for part, and they are lazy by default, doing no work until consumed. Python's generators map to the `Iterator` trait: implement `next()` on a struct for full control, or return `impl Iterator` built from adapters for the everyday case, with `move` closures carrying the captured state out of the function. Finally, closures are values: boxed as `Box<dyn Fn(...)>`, they can live in struct fields and be swapped at construction time — the mechanism Chapter 29 will grow into rustdv's replacement for the UVM factory.

That `Box` in figure 11 was the first time this book put a value on the heap on purpose, and I slipped it past you with one sentence of explanation. It deserves better — because `Box` has two siblings, `Rc` and `RefCell`, and among them they answer the question every UVM engineer eventually asks here: *how do two components share one scoreboard?* Chapter 13 pays that debt.
