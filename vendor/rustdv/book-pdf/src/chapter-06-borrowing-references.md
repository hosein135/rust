# Chapter 6: Borrowing and References

> **In the UVM...** we got a warning instead of a rule. Every dialect has this bug: two processes — a monitor and a consumer — sharing a `transaction_data` variable, both waking on the same rising edge, with no guarantee about which runs first. SystemVerilog's LRM declares the ordering indeterminate and grew `program` blocks and clocking blocks trying to fence testbenches away from the consequences; cocotb's documentation, examining the same temptation around `NullTrigger`, says flatly **Do not do this** — the scheduling order "is not deterministic and should generally not be relied upon" — and tells you to synchronize with an event instead. Both toolchains could warn you. Neither could stop you. This chapter is about the language feature that stops you.

Chapter 5 ended with a mental model we will now spend a whole chapter earning: *ownership is about responsibility, borrowing is about access.* Ownership answered the question the garbage collectors answered silently — who frees this transaction? — by declaring exactly one owner, and by making assignment a *move* of that responsibility. When the monitor handed a transaction to the scoreboard, the monitor was done with it, and the compiler enforced its being done with it.

But that model, taken alone, is unlivable. A scoreboard that must *own* every transaction it merely wants to *look at* would be a scoreboard fed entirely by `clone()` calls. A coverage collector that steals the transaction from the scoreboard is not a testbench, it is a relay race. Most of the time a component does not need responsibility for a value. It needs access. Rust's mechanism for access-without-responsibility is the **reference**, and the act of granting one is called **borrowing** — a word chosen carefully, because a borrow, unlike a Python reference or a SystemVerilog class handle, comes with an obligation to give the value back and rules about what you may do while you hold it.

## Shared references: many readers

A shared reference is written `&T` — "a reference to a T" — and you create one with `&`:

```rust
// Figure 1: Shared references — everyone may look, nobody may touch

struct Transaction {
    data: u8,
}

fn report(t: &Transaction) {
    println!("Saw transaction with data {}", t.data);
}

fn main() {
    let t = Transaction { data: 42 };

    let monitor_view = &t;    // a borrow
    let coverage_view = &t;   // another borrow -- readers may alias freely

    report(monitor_view);
    report(coverage_view);

    println!("Still the owner: {}", t.data);   // t was never moved
}
```

```text
--
Saw transaction with data 42
Saw transaction with data 42
Still the owner: 42
```

(I am reusing the `Transaction` struct from Chapter 5. Structs get their full treatment in Chapter 7; for now it is a labeled box holding a `data` field.)

Three things to notice, each a quiet contrast with Chapter 5. First, `report` takes `&Transaction`, not `Transaction` — so calling it does not move `t`. The function borrows the transaction, reads it, and the borrow ends when the function returns. Second, we made *two* references to `t` at the same time and the compiler did not object: shared references may alias freely, any number of them at once. Third, after all that lending, `t` is still ours to use in the final `println!`. Responsibility never changed hands; only access did.

The price of this generosity is stamped into the type: through a `&T` you may *read* and nothing else. Try `monitor_view.data = 7` and the compiler will refuse — not because `t` is immutable (though it is; we never said `let mut`), but because a shared reference never grants write access, period. Everyone may look. Nobody may touch.

If you want an analogy, `&T` is what every handle in a well-disciplined codebase *pretended* to be: something you pass around for reading, trusting that nobody mutates through it. Rust removes the trust and keeps the handle.

## Exclusive references: one writer

Sometimes touching is the point. A driver that receives a transaction may legitimately need to fill in a timestamp; a BFM may need to update a field before sending. For write access you need the second kind of reference, `&mut T` — an **exclusive reference**:

```rust
// Figure 2: An exclusive reference grants write access

struct Transaction {
    data: u8,
}

fn scramble(t: &mut Transaction) {
    t.data = 99;
}

fn main() {
    let mut t = Transaction { data: 42 };   // mut: the owner permits mutation

    scramble(&mut t);   // lend write access, briefly

    println!("After scramble: {}", t.data);
}
```

```text
--
After scramble: 99
```

Two spellings of `mut` appear here and they are doing different jobs. `let mut t` is the owner declaring that this value may ever be mutated at all — Chapter 3's immutable-by-default rule. `&mut t` is the owner lending *exclusive, writable* access to somebody else. You cannot create a `&mut` borrow of a variable that was not declared `mut`; permission flows downhill from the owner.

"Exclusive" is the load-bearing word. While a `&mut T` exists, it is the *only* live reference to that value — no other `&mut`, no `&`, and the owner itself may not so much as read the value until the exclusive borrow ends. The writer works alone, with the door locked.

## The rule: aliasing XOR mutability

We can now state the rule the whole chapter — arguably the whole language — hangs on. Memorize it in this form:

> **At any moment, a value may have many readers or one writer — never both.**

Rust programmers call this **aliasing XOR mutability**: a value may be aliased (many `&T`), or it may be mutable (one `&mut T`), but never both at once.¹ Everything the borrow checker does is enforcement of this one sentence, plus bookkeeping about *when* each borrow ends.

Why is this the rule worth building a language around? Because every data race you have ever debugged — and every race those warnings at the top of this chapter were about — has the same anatomy: one piece of state, at least one writer, at least one other reader or writer, and no enforced ordering between them. Aliasing XOR mutability makes that anatomy unrepresentable. If there is a writer, there is nobody else; if there is anybody else, there is no writer. The race has no room to exist.

That is a large claim, so let us go back to the scene of the crime.

> ¹ The rule has the same shape as a conference-room whiteboard policy: any number of people may stand and read it, or exactly one person may hold the marker — but a reader standing at a whiteboard while someone else is erasing it learns nothing trustworthy, and everybody knows it.

## The NullTrigger race, rejected at compile time

Recall the pattern every dialect tells you never to write: a shared `transaction_data` variable, a monitor process that writes it, a consumer process that reads it, both triggered by the same clock edge, with nothing but hope deciding who runs first. In SystemVerilog and Python alike, that code runs. It even works, usually, on your simulator, until a scheduler's ordering shifts and it silently doesn't.

We cannot yet write real concurrent tasks in Rust — the executor and `spawn` arrive in Part II — but we do not need them to expose the bug, because the bug was never really about tasks. It was about *two live accessors of one value, one of them a writer, with no enforced order*. We can write exactly that in six lines, giving the monitor its write access and the scoreboard its read access:

```rust
// Figure 3: Two tasks' worth of access to one value -- the borrow checker objects

fn main() {
    let mut transaction_data: Option<u8> = None;

    let monitor = &mut transaction_data;   // the monitor's claim: write access
    let scoreboard = &transaction_data;    // the scoreboard's claim: read access

    *monitor = Some(42);                   // the monitor sees a transaction...

    match scoreboard {                     // ...and the scoreboard checks it
        Some(data) => println!("Scoreboard checking {data}"),
        None => println!("Nothing to check yet"),
    }
}
```

```text
--
error[E0502]: cannot borrow `transaction_data` as immutable because it is also borrowed as mutable
 --> src/main.rs:5:22
  |
4 |     let monitor = &mut transaction_data;
  |                   ---------------------- mutable borrow occurs here
5 |     let scoreboard = &transaction_data;
  |                      ^^^^^^^^^^^^^^^^^ immutable borrow occurs here
...
7 |     *monitor = Some(42);
  |     ------------------- mutable borrow later used here

For more information about this error, try `rustc --explain E0502`.
error: could not compile `borrow_race` (bin "borrow_race") due to 1 previous error
```

(`Option<u8>` is Rust's typed replacement for the value-or-nothing idiom — Python's `None`, SystemVerilog's null or sentinel value. Read `Some(42)` as "has a value" and `None` as "doesn't." Chapter 9 gives `Option` its full due; here it is set dressing.)

Read the error the way Chapter 2 taught you to — as a collaborator's comment, not a rejection slip. The compiler identifies all three actors in the crime: where the mutable borrow was created (line 4), where the immutable borrow was created while the writer still existed (line 5 — that is the actual error, marked with carets), and — this is the part no Python tool could ever tell you — where the writer is *used again afterward* (line 7), proving the two claims overlap in time. Writer alive, reader alive, same value, same moment: aliasing AND mutability. The program does not compile.

Sit with what just happened. This bug has been documented for decades. cocotb's docstring says **Do not do this** in bold; the SystemVerilog LRM calls the ordering indeterminate and leaves you to your fate; a generation of verification leads has taught the discipline that avoids it. And still, nothing in either toolchain would stop a tired engineer from writing it on a Friday afternoon, and nothing would flag it until a scheduler reordering made a regression flicker, weeks later, in someone else's test. Rust converts the whole affair into three seconds of compile time and an error message with line numbers.

## The fix: make the ordering real

The classic fix was an event — SystemVerilog's `->done` and `@(done)`, cocotb's `Event`: the monitor writes, *then* sets the event; the consumer awaits the event, *then* reads. Notice what the event actually contributed — it forced the write and the read into a definite order, so that the writer was finished before the reader began. The borrow checker demands precisely the same thing, and in straight-line code you provide it the same way: sequence the accesses so they do not overlap.

```rust
// Figure 4: The same actors, with the ordering made real

fn main() {
    let mut transaction_data: Option<u8> = None;

    let monitor = &mut transaction_data;
    *monitor = Some(42);                  // the monitor writes...
                                          // ...and its borrow ends at its last use

    let scoreboard = &transaction_data;   // now the reader may claim access
    match scoreboard {
        Some(data) => println!("Scoreboard checking {data}"),
        None => println!("Nothing to check yet"),
    }
}
```

```text
--
Scoreboard checking 42
```

The only change from figure 3 is the order of the lines — the monitor finishes its writing *before* the scoreboard's borrow begins — and that is the entire point. The fix for an unordered write/read conflict is an ordering, in Rust as in Python; the difference is that Rust would not let you skip it.

One subtlety in figure 4 deserves a sentence, because it will save you fights with the compiler later: `monitor`'s borrow ended at its *last use* (the write on the line above), not at the closing brace of its scope. The borrow checker tracks how long each borrow is actually needed, not merely where the variable was declared. If it worked scope-to-brace, figure 4 would not compile either; because it works use-to-use, tidy sequential code like this passes without ceremony.

And one note of caution, so you do not over-generalize: when the monitor and scoreboard become concurrent tasks in Chapter 16, they cannot simply take turns borrowing a local variable — each task needs its own durable handle to shared state, which is exactly the situation `&`/`&mut` alone cannot express. Rust's answers there are the queue (the two tasks never share the value at all — Chapter 16) and, when sharing truly is the design, the `Rc<RefCell<T>>` escape hatch, which moves this chapter's rule from compile time to runtime checking (Chapter 13). Both of those tools are built on top of the rule you just learned, not exemptions from it. The rule is the constant; only the enforcement point moves.

## Lifetimes: recognize the syntax, decline the rabbit hole

There is one more piece of borrowing syntax you will meet in the wild, and I want you to be able to greet it without alarm. Sometimes a function signature carries a small tick-marked annotation:

```rust
fn newer<'a>(x: &'a Transaction, y: &'a Transaction) -> &'a Transaction
```

That `'a` (pronounced "tick-a") is a **lifetime** — a name for *how long a borrow lasts*. Every reference in every program has one; the compiler has been inferring them silently in every figure of this chapter. They surface in the syntax only when the compiler needs your help connecting inputs to outputs: `newer` returns a reference, and the compiler must know whether that returned borrow is tied to `x`, to `y`, or to something else, because whoever *receives* the returned reference is now borrowing from one of them, and the checker must know which value has to stay alive. Writing `'a` on all three says: the returned reference lives no longer than the shorter-lived of the two arguments. That is the whole idea — lifetimes are the paperwork that lets borrow checking work *across* function boundaries.

Here is what you need at this point in the book, in full: recognize `'a` as "a named lifetime," know that it connects the lifetime of an output reference to the lifetimes of input references, and know that when the compiler asks you for one, it is asking "if I hand this reference back to your caller, which argument is it borrowing from?" You will see lifetimes in error messages and in library documentation; you will write them rarely, because the compiler's inference rules cover the overwhelming majority of testbench-shaped code, and rustdv's public API is deliberately designed to keep user-facing lifetimes rare besides.

And here I am going to make an editorial call with precedent: *Python for RTL Verification* taught every Python feature a testbench needed and then, at the door of the metaclass, stopped — noting that pyuvm used them internally but that a testbench author never needed to write one. Lifetimes get the same treatment here, for the same reason. There is real depth behind that tick mark — variance, higher-ranked bounds, a small literature of compiler lore — and none of it, not one line, appears in the testbenches this book builds. When a lifetime annotation is forced on us later, I will explain that occurrence on the spot. Until then: recognize it, read past it, keep going.²

> ² Rust folklore holds that you do not truly understand lifetimes until you have argued with the borrow checker about them for a weekend. This book's position is that your weekends belong to your regressions.

## What borrowing buys the testbench

Step back and look at the shape of what you now know. Ownership (Chapter 5) decides who is responsible for every value — who frees the transaction, in a world with no garbage collector. Borrowing (this chapter) decides who may access it meanwhile, under a rule — many readers or one writer, never both — that turns the shared-state race every dialect warns about into a compile error with line numbers. Together they are the machinery that turns a whole class of 2 a.m. testbench bugs into a red squiggle at your desk before lunch.

So far, though, our transactions have been a struct with one sad little `u8` in it, and our operations have been strings of field pokes. A real TinyALU transaction has two operands and an *operation* — and "an operation" is a value that is exactly one of ADD, AND, XOR, or MUL, a kind of value Python approximated with `IntEnum`, SystemVerilog approximates with `typedef enum`, and Rust does beautifully. Structs, methods, and enums with payloads are Chapter 7, and they are where Rust starts being fun.
