# Chapter 13: Smart Pointers: Box, Rc, and RefCell

> **In the UVM...** we guarded our class internals as best the language allowed. SystemVerilog gave us real keywords — `local` and `protected`, compiler-enforced. Python could not: its classic story is a `Temperature` class and a user who kept reaching past its methods to poke `tt.temp` directly — deterred first by a single underscore (a convention, unenforced), then by double-underscore name mangling (enforced, grudgingly), and finally civilized by `@property` accessors. Both languages were answering the same question: who gets to touch this value, and on whose terms?

Chapters 5 and 6 handed you two rules and told you the rest of the book stands on them: every value has exactly one owner, and at any moment a value may have many readers or one writer, never both. Since then, you may have been quietly carrying a worry. You have written UVM testbenches. You *know* what one looks like inside: an environment holding an agent, a monitor and a scoreboard both holding the same BFM or the same virtual interface, an analysis port fanning one transaction out to three subscribers. Shared access everywhere, and — be honest — shared *mutable* access more often than the diagrams admit. If Rust's rules are absolute, how does any of that survive the port?

The answer is that the rules are absolute but the *enforcement point* is negotiable. Rust provides a small set of standard-library types — the community calls them **smart pointers** — that let you buy back Python-style sharing, one capability at a time, each purchase visible in your code and each with a price tag attached. This chapter teaches the three you will actually meet: `Box<T>`, `Rc<T>`, and `RefCell<T>`. By the end you will be able to read `Rc<RefCell<T>>` and recognize an old friend wearing a name tag: a Python object reference, made visible.

## `Box<T>`: the heap, on request

The simplest smart pointer barely earns the name. `Box<T>` puts a value on the heap and owns it — one owner, moved and dropped exactly like everything in Chapter 5. The only thing that changed is *where the bytes live*.

```rust
// Figure 1: A Box owns its value on the heap — everything else is Chapter 5

struct Transaction {
    a: u8,
    b: u8,
    op: u8,
}

fn main() {
    let t = Box::new(Transaction { a: 5, b: 3, op: 1 });
    println!("boxed transaction: {} op {}", t.a, t.b);
}   // t goes out of scope; the Box is dropped; the heap memory is freed.
    // One owner, one drop, on time — nothing new.
```

```text
--
boxed transaction: 5 op 3
```

Notice `t.a` works without ceremony — a `Box` hands through field access as if the box weren't there. So why would you ever ask for one? In Python, and in SystemVerilog's class world, you never chose; every object lived on the heap and every variable was a handle to it. Rust defaults to the stack and lets you opt into the heap, and there are two situations where you must. The first is a type whose size the compiler cannot pin down — a recursive type, like a linked list whose node contains another node; boxing the inner node gives the compiler a fixed-size pointer to reason about. The second you have already met: **trait objects**. Chapter 10 introduced `Box<dyn Component>` as the way to store "some type implementing `Component`, decided at runtime" — and since the compiler cannot know that type's size, onto the heap it goes, behind a `Box`.

That is the whole story. `Box<T>` is single ownership with a heap address. When you see one in rustdv — the boxed component a factory maker returns, say — read it as "an owned value whose concrete type or size wasn't known at compile time" and move on. The interesting escape hatches are the next two.

## `Rc<T>`: Python's refcount, made visible

Here is a piece of CPython trivia that is about to stop being trivia: every object you ever created in Python carried a hidden integer — a **reference count** — and every assignment, argument pass, and container insert incremented it, while every scope exit and `del` decremented it. When the count hit zero, the object died.¹ You never saw this machinery; it ran on every single Python statement you ever executed, silently, whether you needed it or not. We can even catch it in the act:

```python
# Figure 2: The refcount Python never showed you

import sys

a = ["ADD", 5, 3]
print(sys.getrefcount(a))
b = a
print(sys.getrefcount(a))
```

```text
--
2
3
```

(`getrefcount` reports one higher than you'd guess, because the act of calling it lends the list one more temporary reference. Even the inspection tool participates in the scheme.)

Rust's `Rc<T>` — *reference counted* — is exactly this mechanism, with two differences: you opt into it per value instead of paying for it everywhere, and the increments happen only where you can see them. `Rc::new` wraps a value and starts the count at one. `Rc::clone` does *not* copy the value — it copies the *handle* and increments the count, which is precisely what Python's `b = a` did behind your back. When each `Rc` handle is dropped, the count decrements; at zero, the value is dropped. It is CPython's memory management, offered à la carte.

```rust
// Figure 3: Rc::clone increments a refcount — on purpose, where you can see it

use std::rc::Rc;

fn main() {
    let bfm = Rc::new(String::from("TinyALU BFM"));
    println!("owners: {}", Rc::strong_count(&bfm));

    let driver_handle = Rc::clone(&bfm);
    {
        let monitor_handle = Rc::clone(&bfm);
        println!("owners: {}", Rc::strong_count(&bfm));
        println!("monitor sees: {monitor_handle}");
    }   // monitor_handle dropped here: count decrements

    println!("owners: {}", Rc::strong_count(&bfm));
    println!("driver sees: {driver_handle}");
}
```

```text
--
owners: 1
owners: 3
monitor sees: TinyALU BFM
owners: 2
driver sees: TinyALU BFM
```

Read figure 3 as a negotiation with Chapter 5. That chapter's rule — one value, one owner — has not been repealed; it has been *satisfied cleverly*. The `Rc` bookkeeping block is the value with the single owner story; the handles share it, and "who frees this?" is answered mechanically: the last handle out turns off the lights. This is why the method is spelled `Rc::clone(&bfm)` rather than hiding inside an assignment — the Rust convention is to make every refcount bump greppable, so that when you audit a testbench you can find each place ownership got shared and ask whether it earned its keep. Python bumped refcounts on every line and could not tell you where; Rust bumps them only where the code says `Rc::clone`.

> ¹ Mostly. CPython also runs a cycle-detecting garbage collector to rescue objects that point at each other and hold their mutual counts above zero forever. Hold that thought — `Rc` has the same weakness and no rescue squad, and it is going to matter later in this chapter.

So can several components share a BFM this way? Almost. There is a catch, and it is the same catch Chapter 6 stamped into `&T`: sharing is a *reader's* privilege.

```rust
// Figure 4: Shared owners are readers — Rc will not hand out write access

use std::rc::Rc;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = Rc::new(Scoreboard { errors: 0 });
    let handle = Rc::clone(&sb);
    handle.errors += 1;
    println!("errors: {}", sb.errors);
}
```

```text
--
error[E0594]: cannot assign to data in an `Rc`
 --> src/main.rs:9:5
  |
9 |     handle.errors += 1;
  |     ^^^^^^^^^^^^^^^^^^ cannot assign
  |
  = help: trait `DerefMut` is required to modify through a dereference,
          but it is not implemented for `Rc<Scoreboard>`
```

Of course it refused. Two handles alive means two potential accessors, and *many readers or one writer, never both* applies to shared owners exactly as it applied to references. `Rc<T>` gives you Python's sharing but only the reading half of Python's behavior. For the writing half, we need the strangest and most instructive type in this chapter.

## `RefCell<T>`: the borrow checker, moved to runtime

Everything the borrow checker has done so far, it has done at compile time, by proving things about your source code. But some sharing patterns are true in ways a compiler cannot see from the source — *these two components take turns, I promise* — and for those, Rust offers a deal: **`RefCell<T>` keeps the borrowing rules but checks them at runtime.** Same law, different courtroom.

A `RefCell<T>` wraps a value and replaces `&`/`&mut` with two accessor methods: `borrow()` returns a read handle, `borrow_mut()` returns a write handle, and the cell *counts* its outstanding loans. Many readers, or one writer — enforced by a counter at the door instead of a proof in the compiler.

```rust
// Figure 5: Interior mutability — mutation through an immutable binding

use std::cell::RefCell;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = RefCell::new(Scoreboard { errors: 0 });   // note: no `mut`

    sb.borrow_mut().errors += 1;    // the write handle lives for this line only

    println!("errors: {}", sb.borrow().errors);
}
```

```text
--
errors: 1
```

Look hard at the first line of `main`: there is no `mut`. Since Chapter 3, that has meant untouchable — yet we mutated `errors` anyway. This trick has a name, **interior mutability**: from the outside, `sb` is an immutable value you can share freely; on the inside, the cell hands out exclusive write access to one caller at a time, checked at the moment of the call. If that sounds familiar, it should — it is the gatekeeper move from this chapter's opening, replayed at the level of the type system. Python's `Temperature` class hid `__temp` behind properties, and SystemVerilog's `local` fields hid behind accessor functions, so the class could enforce its rules at every access; `RefCell` hides its value behind `borrow` and `borrow_mut` so the *borrowing* rules get enforced at every access. All three designs answer the same question — who gets to touch this value, and on whose terms? — by making traffic pass through a gatekeeper. The difference is what the gatekeeper checks: your old accessors checked whatever validation you wrote by hand; `RefCell` checks aliasing XOR mutability, the one rule this whole language is built on.

And what happens when the rule is violated? At compile time, a violation was a program that never existed. At runtime, the program exists — it is halfway through a simulation — so the only honest response left is to stop:

```rust
// Figure 6: The borrow checker at runtime — a panic replaces the compile error

use std::cell::RefCell;

struct Scoreboard {
    errors: u32,
}

fn main() {
    let sb = RefCell::new(Scoreboard { errors: 0 });

    let reader = sb.borrow();          // a reader is at the whiteboard...
    let mut writer = sb.borrow_mut();  // ...and a writer grabs the marker

    writer.errors += 1;
    println!("reader saw: {}", reader.errors);
}
```

```text
--
thread 'main' panicked at src/main.rs:12:25:
already borrowed: BorrowMutError
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Set figure 6 next to Chapter 6's figure 3. They are the *same program* — one value, a live reader, a live writer, overlapping. In Chapter 6 the compiler rejected it with error E0502 and three annotated line numbers, before anything ran. Here it compiled without a murmur and then panicked at line 12, at runtime. That is the entire trade, in two figures: `RefCell` does not weaken the rule — a reader and a writer still cannot coexist, and the race Chapter 6 buried stays buried — but it moves the *discovery* of a violation from your desk to the running program. In a testbench, "the running program" means seed 8,441, forty minutes in, with the license checked out. You have not escaped the borrow checker. You have volunteered to meet it later, at a worse time, with a stack trace instead of a source annotation.²

That framing tells you exactly when `RefCell` is legitimate: when you *know* the accesses cannot overlap — because your components take turns at await points, because the write handle lives for one expression as in figure 5 — but the knowledge lives in the design rather than in anything the compiler can verify from the source. Then the runtime check is not a time bomb; it is an assertion, permanently guarding an invariant you believe, and the panic is the assertion firing on the day you turn out to be wrong.

> ² A panic in a rustdv task does not take down the simulator, for the record — it is caught at the task boundary and scored as a test failure, the same way cocotb catches a stray exception per task. Cold comfort at seed 8,441, but comfort.

## `Rc<RefCell<T>>`: a Python reference, made visible

Now stack them. `Rc` gave us shared ownership with read-only access; `RefCell` gave us gate-checked mutation of a shared value. Compose them — `Rc<RefCell<T>>` — and you have shared handles, any of which can mutate the value, with the borrow rules enforced at each access. Which is to say: you have rebuilt the Python object reference, the thing every variable in every Python program you ever wrote actually was.

Chapter 5 opened with a Python figure that proved `b = a` gives two names for one mutable object, and then showed Rust refusing to compile the same experiment. We have been in debt to that figure for eight chapters. Time to pay it off:

```rust
// Figure 7: Chapter 5's Python experiment, finally legal in Rust — with its costs itemized

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Transaction {
    op: String,
    a: u8,
    b: u8,
}

fn main() {
    let a = Rc::new(RefCell::new(Transaction {
        op: String::from("ADD"),
        a: 5,
        b: 3,
    }));
    let b = Rc::clone(&a);          // two names...

    b.borrow_mut().op = String::from("MUL");   // ...mutate through one...

    println!("{:?}", a.borrow());              // ...observe through the other
    println!("same object: {}", Rc::ptr_eq(&a, &b));
}
```

```text
--
Transaction { op: "MUL", a: 5, b: 3 }
same object: true
```

Line for line, this is Chapter 5's figure 1: two names, one object, a mutation through `b` visible through `a`, and `Rc::ptr_eq` standing in for Python's `is`. What Python and SystemVerilog handles gave you invisibly and unconditionally, Rust sells you piecewise, each purchase named in the source: `Rc::new` (this value will be shared), `Rc::clone` (here is another owner), `borrow_mut()` (I want the marker, check me at the door), `borrow()` (just reading, count me). A reviewer can see every one of those decisions. In your old languages there was nothing to see — which is precisely why Python needed conventions begging users not to touch `_temp`, and why SystemVerilog grew the `local` keyword.

## The bill

I owe you the honest price list, because "just wrap it in `Rc<RefCell<>>`" is the single most common way for a new Rust programmer to dig a hole.

**Runtime checks that can panic.** Every `borrow()` and `borrow_mut()` is a check that can fail, and figure 6 showed what failure looks like: a panic, at runtime, in whatever seed happens to trip it. The compile-time guarantee you have enjoyed since Chapter 6 is gone for this value; you are back to *testing* for the bug instead of being proven free of it.

**Bookkeeping overhead.** Refcount increments, borrow-flag checks — each is tiny, and unlike Python you pay only on the values you wrapped. But "tiny" is a per-access word, and monitors touch shared state per transaction, per seed, per regression. It adds up; budget for it consciously.

**Reference cycles leak.** Here the footnote from earlier comes due. CPython backs its refcounts with a cycle-collecting garbage collector; `Rc` has no such backstop. If a parent holds an `Rc` to its child and the child holds an `Rc` back to the parent, the counts never reach zero and the memory never frees — a leak, silent and permanent. (The standard library's `Weak` type exists to break such cycles; rustdv's design never needs it, for reasons one section away, so this book leaves it at a mention.)

**The temptation.** This is the real cost. Once you know `Rc<RefCell<T>>` exists, every borrow-checker error acquires an easy exit: wrap it, clone it, move on. Do that habitually and you will have written Python with worse syntax — every shared value a potential runtime panic, every refcount a small tax, and the compiler's proofs traded away for the debugging sessions this book keeps promising you left behind. The discipline is the one you already know from private attributes, pointed the other way: reach for the escape hatch deliberately, at a designed boundary, and let the default remain the default.

## Why rustdv barely needs any of this

Which brings us to the question this chapter has been building toward: how much `Rc<RefCell<T>>` will the testbenches in the rest of this book actually contain? You know how the UVM is built — in SystemVerilog and pyuvm alike, every child holds a handle to its parent, every parent holds its children, and a global tree (`uvm_root`'s registry, pyuvm's `component_dict`) holds a reference to everything. That is a graph full of exactly the parent-and-child cycles that make `Rc` leak, and it works in your old languages only because a garbage collector untangles what refcounts cannot. Port that structure naively and you would be signing up for `Rc<RefCell<T>>` everywhere, `Weak` back-references to break the cycles, and a runtime borrow check on every component access — the old architecture with Rust's ceremony, the worst of both worlds.

rustdv's design refuses the premise: **the component tree *is* the ownership tree**. An environment is a struct; its children are its fields; the parent owns them the way any struct owns its fields, and drops them at its own drop, in the way you have understood since Chapter 5. There is no back-pointer to a parent, no global registry, no cycle — so there is nothing for a refcount to get wrong and no shared mutation for a `RefCell` to referee. When the monitor needs to hand the scoreboard a transaction, it will not reach through a shared reference to poke scoreboard state; it will send the transaction down a channel, moving ownership the way Chapter 5's baton pass always wanted to. The hierarchy that forced the earlier dialects into pervasive implicit sharing simply is not shaped that way here.

What survives is the truly shared resource — the thing that really does have several users and really is one object. The canonical example, previewed now and built in Chapter 25: one `TinyAluBfm` wrapping the DUT interface, needed by a driver *and* a monitor at once. SystemVerilog engineers should hear "virtual interface in the config database" — the same job, and rustdv solves it the same way: the test creates the BFM once and files an `Rc<TinyAluBfm>` in the ConfigDb, and each component that needs it retrieves a counted handle by name. Notice which half of this chapter that uses: `Rc` alone, no `RefCell`, because the BFM's async methods take `&self` and the sharing is read-shaped from the outside. Shared mutability, where it appears in rustdv at all, is deliberate, boundary-marked, and rare — every use called out in the design rather than ambient in the architecture. The escape hatch exists, you now know exactly what it costs, and the framework's job is to make sure you almost never reach for it.

That completes the Rust you need for values: how they are owned, borrowed, shaped, collected, and — this chapter — shared on purpose. What you do not yet know is how Rust programs are *organized*: where `use std::rc::Rc` has been coming from all this time, what a crate actually is, and how `cargo` turns a directory of files into the testbench library a simulator can load. Chapter 14 is about modules, crates, and cargo — and it ends with something neither of your languages ever gave you: unit tests that run in milliseconds, no simulator required.
