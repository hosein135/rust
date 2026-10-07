# Chapter 5: Ownership

Every chapter so far has had a twin in the languages you know. `let` had assignment, `match` had `case` and `elif` chains, `u8` had `byte`. This chapter has no twin, because it answers a question neither of your languages ever let you hear: *when a value's time is up, who is responsible for destroying it?*

You have created millions of objects — transactions, components, queues full of commands — and you have never once destroyed one. You didn't have to. Python's garbage collector followed you around the testbench like a diligent stagehand, watching which objects still had names pointing at them and quietly disposing of the ones that didn't;¹ SystemVerilog's runtime does the same for class objects, keeping each one alive until its last handle drops. Both stagehands do the job so well, and so silently, that most programmers never learn the job exists.

Rust has no garbage collector. There is no stagehand. And yet Rust programs do not leak memory, do not free things twice, and do not touch things after they're freed — the classic sins of C. Rust pulls this off with one idea, enforced by the compiler, and that idea is the hinge of the entire language: **ownership**. Chapter 1 promised that the rules behind Rust's no-runtime-cost safety would bend your brain exactly once. This is the chapter where the bending happens.

> ¹ CPython's stagehand is mostly a reference counter — every object carries a count of the names and containers pointing at it, and hits the trash at zero — with a cycle-detecting garbage collector mopping up the cases where two objects point at each other and the counts never fall. The details don't matter here; the silence does.

## Who frees this? The old answer

Let's watch the stagehand work. In Python, a variable is not a box holding a value — it is a name tag stuck onto an object that lives somewhere on the heap. Assignment copies the name tag, never the object — exactly as assigning one SystemVerilog class handle to another copies the handle, never the object. In figure 1, `a` and `b` are two tags on one transaction-ish list, which we can prove by mutating through one name and looking through the other.

```python
# Figure 1: Python assignment: two names, one object

a = ["ADD", 5, 3]
b = a
b[0] = "MUL"
print(a)
print(a is b)
```

```text
--
['MUL', 5, 3]
True
```

You knew this, in whichever spelling: every UVM component holding a handle to the BFM, every scoreboard holding the same transaction object the monitor held. The part you may never have said out loud is the lifetime question: that list stays alive as long as *any* tag points at it, and it dies whenever the last tag disappears, at a moment of the runtime's choosing. Nobody owns the list. Ownership is smeared across every name that ever touched it, and the runtime keeps the books.

For testbenches this policy is comfortable right up until it isn't: the monitor that holds a stale handle to a component the test rebuilt, the two subscribers that received the "same" transaction and one of them mutated it, the destructor that runs at a time no document will commit to. The garbage-collected answer to "who frees this?" is *nobody in particular, eventually*. Rust's answer is one word long.

## Rust's answer: one owner

Here is the rule the rest of this book stands on:

> Every value in Rust has exactly one **owner** — the variable (or, later, the struct field) responsible for it. When the owner goes out of scope, the value is destroyed, immediately. Assignment doesn't copy a name tag; it **moves** ownership to the new variable, and the old one is dead.

That last clause is the shock. Let's take the shock now, on purpose, with the compiler watching. Figure 2 is the two-names experiment from figure 1, translated into Rust.

```rust
// Figure 2: Assignment moves — and the old name is gone

fn main() {
    let a = String::from("ADD 5 3");
    let b = a;
    println!("{a}");
    println!("{b}");
}
```

```text
--
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:4:15
  |
2 |     let a = String::from("ADD 5 3");
  |         - move occurs because `a` has type `String`, which does not
  |           implement the `Copy` trait
3 |     let b = a;
  |             - value moved here
4 |     println!("{a}");
  |               ^^^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
3 |     let b = a.clone();
  |              ++++++++
```

Read that error the way Chapter 2 taught you, because it is one of the best-written error messages in any compiler and it narrates the whole story. Line 2: the string was created and `a` owned it. Line 3: `value moved here` — the assignment handed ownership to `b`, and `a` stopped being a valid name for anything. Line 4: we tried to use `a` after the move, and the compiler refused to build the program.

Sit with what did *not* happen. The program did not run and print something surprising. It did not crash at 2 a.m. in seed 8,441 of a regression. It never existed as a program at all. In your old languages, `b = a` gives you two live names and a shrug about lifetimes; in Rust, `let b = a;` is a baton pass — after it, exactly one variable is responsible for that string, and the compiler will name the exact line where responsibility changed hands. One value, one owner, at every moment, provable at compile time. That invariant is the entire trick, and everything Rust does that no garbage-collected language can — no GC, no data races, deterministic cleanup — falls out of it.

Notice, too, the compiler's parting suggestion: `a.clone()`. It has read your mind — or at least your options. We'll take it up on that shortly.

## Scope is the destructor

If every value has exactly one owner, then "who frees this?" has a mechanical answer: the owner does, at the moment it goes out of scope. Rust calls this **dropping** the value, and it is as deterministic as the closing brace it happens at.

```rust
// Figure 3: Values die at the closing brace — every time, on time

fn main() {
    {
        let cmd = String::from("MUL 7 6");
        println!("inside the scope: {cmd}");
    }   // <- cmd's owner goes out of scope RIGHT HERE.
        //    The String is dropped, its memory freed, before
        //    the next line runs. No collector. No "eventually."
    println!("after the scope");
}
```

```text
--
inside the scope: MUL 7 6
after the scope
```

The output is unremarkable; the guarantee is not. That string's memory was returned *at the brace* — not at the next garbage-collection pause, not when a reference count happened to hit zero, not "at interpreter shutdown, probably." If you have ever tried to close a file, flush a log, or release a queue in a Python `__del__` method, you know what that "probably" costs: the language reference makes carefully hedged promises about when `__del__` runs, and none at all in some shutdown cases.² Rust replaces the hedging with a rule you can point to in the source code. From Chapter 24 on, this rule will be doing serious work — an objection guard that ends a run phase by dropping at the closing brace, tasks whose cleanup runs the instant they're cancelled — but the whole mechanism is already in front of you in figure 3. Deterministic destruction isn't a feature bolted onto ownership. It *is* ownership, viewed from the value's last moment.

> ² The Python language reference notes that it is "not guaranteed" that `__del__` will be called for objects still alive when the interpreter exits — one of the great load-bearing "not guaranteed"s of our time.

## Wait — my integers have been fine

A fair objection: you've been assigning integers back and forth since Chapter 3 without the compiler saying a word about moves. Figure 4 confirms it.

```rust
// Figure 4: Copy types don't move — small values are simply copied

fn main() {
    let a: u8 = 5;
    let b = a;              // copies the byte; a is still alive
    println!("a = {a}, b = {b}");
}
```

```text
--
a = 5, b = 5
```

The error message in figure 2 quietly explained why: the string moved *"because `a` has type `String`, which does not implement the `Copy` trait."* Types whose values are small, fixed-size, and self-contained — `u8`, `u16`, `bool`, the TinyALU's operands, all the scalars from Chapter 3 — are **`Copy`** types: assignment duplicates the bits, both variables live, nothing to negotiate. There is no shared object for two names to fight over, so ownership has nothing to protect. (Traits, including how a type comes to be `Copy`, are Chapter 10's business.)

Move semantics governs everything else: `String`, the collections coming in Chapter 8, and — most importantly for us — the structs we build ourselves. Which brings us to the reason this chapter exists.

## The monitor and the scoreboard

Every mechanism in this chapter has been abstract enough to shrug at. So let's make it a testbench problem — *the* testbench problem, the one you've written a dozen times: a monitor observes a transaction on the TinyALU's command bus and hands it to a scoreboard.

We need a transaction type. Structs get their own chapter (Chapter 7); for today, read the first four lines of figure 5 as "a class with only data and no methods" — fields, types, no ceremony. The `op` field really wants to be a proper enum, and in Chapter 7 it becomes one; a `u8` stands in for now. And we need the two parties: a `scoreboard` function that takes a `Transaction` *by value*, and a `main` that plays the monitor. In the UVM these would be components connected by an analysis port; here in our cargo playground, two functions are enough to expose the question that matters.

```rust
// Figure 5: The monitor hands off a transaction — and learns what "hands off" means

struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn scoreboard(t: Transaction) {
    println!("scoreboard checking: {} op {} (code {})", t.a, t.b, t.op);
}   // <- t dropped here: the scoreboard owned it, the scoreboard's
    //    scope ends, the transaction is destroyed. Question answered.

fn main() {
    // main is playing the monitor today.
    let t = Transaction { a: 5, b: 3, op: 1 };
    scoreboard(t);
    println!("monitor logging: a was {}", t.a);
}
```

```text
--
error[E0382]: borrow of moved value: `t`
  --> src/main.rs:15:44
   |
13 |     let t = Transaction { a: 5, b: 3, op: 1 };
   |         - move occurs because `t` has type `Transaction`, which
   |           does not implement the `Copy` trait
14 |     scoreboard(t);
   |                - value moved here
15 |     println!("monitor logging: a was {}", t.a);
   |                                           ^^^ value borrowed here after move
   |
note: consider changing this parameter type in function `scoreboard` to
      borrow instead if owning the value isn't necessary
help: consider cloning the value if the performance cost is acceptable
```

Passing a value to a function moves it, exactly as assignment did — `scoreboard(t)` is a baton pass, and line 15 is the monitor trying to run the next leg without the baton.

Here is what I want you to see: **the compiler is not reporting a syntax mistake. It is asking you a design question.** When the monitor hands this transaction to the scoreboard, what *should* happen? In the UVM you never had to decide. The analysis port handed every subscriber the same handle to the same object, ownership belonged to nobody, and the design question got answered by accident — which worked fine until one subscriber mutated the transaction another was still reading, a bug both earlier books could only warn you about. Rust makes you answer on purpose, and there are exactly two honest answers.

**Answer one: it's a true handoff.** The monitor's job was to observe the transaction and pass it on; it has no business touching it afterward. Then the code is wrong and the compiler is right — delete line 15, and the program compiles. Ownership flows monitor → scoreboard, the scoreboard checks it, and when the scoreboard's scope ends the transaction is dropped, on time, by its one owner. The comment on `scoreboard`'s closing brace in figure 5 is the answer to this chapter's title question, sitting in plain sight.

**Answer two: the monitor still needs it** — to log it, to hand a second copy to a coverage collector. Then the monitor must keep *a real copy*, and Rust has an explicit word for that.

## `.clone()`: copies you can see

The compiler suggested it twice, so let's take the hint. Adding `#[derive(Clone)]` above the struct asks the compiler to write a field-by-field copy routine for us (that one magic line gets a full explanation in Chapters 10 and 21; for now, "please generate the copying code" is the whole story). Then `.clone()` makes an independent duplicate wherever we ask.

```rust
// Figure 6: The monitor keeps a copy — explicitly

#[derive(Clone)]
struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn scoreboard(t: Transaction) {
    println!("scoreboard checking: {} op {} (code {})", t.a, t.b, t.op);
}

fn main() {
    let t = Transaction { a: 5, b: 3, op: 1 };
    scoreboard(t.clone());   // the scoreboard owns the copy...
    println!("monitor logging: a was {}", t.a);   // ...the monitor owns the original
}
```

```text
--
scoreboard checking: 5 op 3 (code 1)
monitor logging: a was 5
```

Two transactions now exist, each with exactly one owner, each dropped at its own owner's closing brace. No sharing, no aliasing, no way for the scoreboard's copy and the monitor's original to interfere. And crucially: mutation of one can never surprise the other — the "two subscribers, one mutated object" bug is not merely discouraged, it is unrepresentable in this code.

It's worth pausing on why Rust makes you *spell out* the copy. In Python, whether an assignment copied or aliased depended on the type — and `copy.deepcopy` existed for the cases you usually met through a bug. SystemVerilog engineers are ahead here — SV never copied a class object on assignment either, and the UVM made you call `clone()` in ink. Rust agrees with that instinct and adds the enforcement: trivial bit-copies (`Copy` types) are silent because they cannot matter; every copy that allocates or duplicates real state is written `.clone()`, visible in the diff, greppable in the review. When a testbench is cloning a million transactions a second, you can *find every clone* and decide whether each one earns its cost.

One more note while the handoff is fresh. When components exchange transactions later in the book, the exchange takes the transaction by value: a `put` into a TLM FIFO (Chapter 31) is a move, and so is a sequence's `finish_item` (Chapter 36) — the monitor-to-scoreboard baton pass, made into infrastructure. The decision you just made line by line (hand it off, or clone and keep one?) is the same decision you'll make at every port, and the compiler will hold you to your answer every time.

## The mental model

There is one thread left hanging, and the compiler dangled it in figure 5's output on purpose: *"consider changing this parameter type in function `scoreboard` to borrow instead if owning the value isn't necessary."* Because ask yourself: does a scoreboard need to *own* the transaction? It needs to read the fields, compare against a prediction, and be done. Taking ownership just to look at something is like requiring the title to a car in order to check its odometer. Rust has a mechanism for looking without taking, written `&`, and it is called borrowing; it is the whole subject of Chapter 6, and I will not steal that chapter's material beyond telling you it exists.

What I will do is hand you the sentence this book will use, from here to testbench 8.0, whenever an API makes you choose between taking a value and referencing it. Say it with me, and keep it:

> **Ownership is about responsibility. Borrowing is about access.**
>
> Own a value when you are responsible for it — for its storage, its lifetime, its eventual destruction, its answer to "who frees this?" Borrow a value when you merely need access to it for a while — to read it, or briefly change it — while responsibility stays exactly where it was.

Every design decision ahead of us is an application of that sentence. The component hierarchy of Chapter 24 works because parents *own* their children — responsibility flows down the tree. FIFOs move transactions because a handoff transfers *responsibility*. Monitors will hand scoreboards references or clones depending on whether the scoreboard needs *access* or needs to *keep* something. When you're stuck on a compiler error anywhere in this book, ask the sentence's question first: does this code need responsibility for the value, or just access to it? The answer usually types itself.

You now hold half the model — the responsibility half. Chapter 6 supplies the access half: `&`, the borrow checker Chapter 1 warned you about, and the rule that lets Rust catch at compile time a race condition earlier testbenches could only fear.
