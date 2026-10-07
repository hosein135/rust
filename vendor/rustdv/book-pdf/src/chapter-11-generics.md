# Chapter 11: Generics

Chapter 10 ended with a promissory note. We wrote `fn check_in<T: Animal>(animal: &T)`, waved at the angle brackets, and promised that generics — code with type parameters, resolved at compile time — would get their own chapter. This is that chapter. By the end of it you will know what the compiler actually does with `<T: Animal>`, why the result runs exactly as fast as code without it, and why `Driver<REQ, RSP>` is, as promised, the most honest thing a driver has ever said about itself.

Here is the small confession first: you have been using generics since Chapter 8. `Vec<T>` is a generic struct. So are `Option<T>` and `Result<T, E>` — every time you wrote `Vec<AluCommand>` or `Result<u16, AluError>`, you were filling in someone else's type parameters. This chapter teaches you to write your own.

> **In the UVM...** we had two answers to "one definition, many types." SystemVerilog's answer was the parameterized class — `class register #(type T = int);` — the machinery beneath every `uvm_sequence #(REQ, RSP)` and `uvm_sequencer #(REQ)` you have ever declared. Python's answer was to skip the question: functions accept everything, a scoreboard's `write()` took whatever the analysis port delivered, and if the object had the right attributes, everything worked. That philosophy has a name, *duck typing* — if it walks like a duck and quacks like a duck, treat it as a duck — and nobody checks for feathers until the moment of the quack.

Duck typing is pleasant to write. Its cost is *when* the feather-check happens: at every call, at runtime, and only on the paths your test actually exercised. Rust keeps the pleasant part — one function, many types — and moves the check: if it implements the trait bound, it's a duck, and you find out at compile time, for every path, including the ones your test forgot. SystemVerilog readers, meanwhile, should read this chapter as *parameterized classes with a real contract* — your instinct is right, and the differences are the good part.

## A function for any type

Suppose we want the largest value in a slice. For the TinyALU's `u8` operands we could write it directly, but the moment we also want the largest `u16` result, or the alphabetically last test name, we are copying the function and changing one type annotation — exactly the copy-and-modify busywork object-oriented programming exists to avoid. Rust's answer is a **type parameter**: a placeholder type, declared in angle brackets, that the caller fills in.

Let's write it the way you would naively write it, because the failure is the lesson.

```rust
// Figure 1: A generic function, first attempt — the compiler wants proof

fn largest<T>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}
```

```text
--
error[E0369]: binary operation `>` cannot be applied to type `&T`
 --> src/main.rs:4:17
  |
4 |         if item > largest {
  |            ---- ^ ------- &T
  |            |
  |            &T
  |
help: consider restricting type parameter `T`
  |
1 | fn largest<T: std::cmp::PartialOrd>(list: &[T]) -> &T {
  |             ++++++++++++++++++++++
```

Read the declaration first: `fn largest<T>` says "this function is defined for some type T, to be named later," and then `&[T]` and `&T` use the placeholder as if it were a real type. Python would have shrugged and run this. Rust refuses, and its objection is philosophically the whole chapter: *you said T could be any type, and then you compared two of them with `>`. Not every type can do that.* What do we know about a type we know nothing about? Nothing — so we may call nothing on it.

The fix is in the compiler's own `help` text, and it is Chapter 10's vocabulary: a **trait bound**. `T: PartialOrd` narrows "any type" to "any type that implements `PartialOrd`" — the ordering trait from Chapter 10's dunder table, the one behind `<` and `>`.

```rust
// Figure 2: The bound is the fix — and the documentation

fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    let operands: Vec<u8> = vec![0x22, 0xAA, 0x07];
    let results: Vec<u16> = vec![0x0154, 0x7100, 0x00FF];
    let tests = vec!["alu_add_test", "alu_xor_test", "alu_mul_test"];

    println!("largest operand: 0x{:02x}", largest(&operands));
    println!("largest result:  0x{:04x}", largest(&results));
    println!("last test name:  {}", largest(&tests));
}
```

```text
--
largest operand: 0xaa
largest result:  0x7100
last test name:  alu_xor_test
```

One function body, three element types, and no type mentioned at any call site — the compiler infers `T` from the argument, the same way it has been inferring types for you since Chapter 3.¹ Notice too that we return `&T`, a reference, not `T`: Chapter 5 taught us that returning the value itself would mean moving it out of the caller's slice, and the borrow is both cheaper and honest about who still owns the data.

The bound does double duty, and this is worth slowing down for. To the *compiler* it is a permission slip: inside `largest`, exactly the methods of `PartialOrd` may be called on `T`, no more. To the *reader* it is documentation you can trust: the signature `fn largest<T: PartialOrd>(list: &[T]) -> &T` tells you everything this function will ever demand of your type. Python's equivalent contract lived in the docstring, the tribal knowledge, or the stack trace; SystemVerilog's lived in an elaboration error that erupted from the class's guts and named your instantiation instead of the requirement.

## More than one requirement

A bound can require several traits at once, joined with `+`. Here is a function a scoreboard might want — compare an expected transaction against an actual one, and complain legibly on a mismatch. Comparing needs `PartialEq`; complaining legibly needs `Debug`. Both go in the contract:

```rust
// Figure 3: Multiple bounds with a where clause

use std::fmt::Debug;

fn check_match<T>(expected: &T, actual: &T) -> bool
where
    T: PartialEq + Debug,
{
    if expected == actual {
        true
    } else {
        println!("MISMATCH: expected {:?}, actual {:?}", expected, actual);
        false
    }
}

fn main() {
    check_match(&0x54u16, &0x54u16);
    check_match(&0x54u16, &0x55u16);
}
```

```text
--
MISMATCH: expected 84, actual 85
```

We could have written `fn check_match<T: PartialEq + Debug>(...)` and it would mean the same thing; the `where` clause is the same information moved out of the angle brackets so the signature stays readable. Convention, and this book, use the inline form for one short bound and `where` for anything longer. Either way, look at what this function is: the duck-typed checker — "anything with an `__eq__` and a `__repr__`," or in SV terms, anything whose base class promised `do_compare()` and `convert2string()` — with the requirements written down and enforced per type. A transaction type that forgot to derive `PartialEq` doesn't fail in hour three of a regression; it fails to compile, with an error pointing at the missing derive.

## Generic structs

Type parameters work on structs the same way, and you already know the syntax from the consumer side — `Vec<T>` — so producing one holds no surprises. Here is a register model wide enough for any leg of the TinyALU:

```rust
// Figure 4: A generic struct — one definition, many widths

struct Register<T> {
    value: T,
}

impl<T: Copy> Register<T> {
    fn new(value: T) -> Self {
        Register { value }
    }
    fn read(&self) -> T {
        self.value
    }
    fn write(&mut self, value: T) {
        self.value = value;
    }
}

fn main() {
    let mut a_reg: Register<u8> = Register::new(0x00);   // the A leg is 8 bits
    let mut result: Register<u16> = Register::new(0x0000); // the result is 16

    a_reg.write(0xAA);
    result.write(0x7100);
    println!("A: 0x{:02x}  result: 0x{:04x}", a_reg.read(), result.read());
}
```

```text
--
A: 0xaa  result: 0x7100
```

Two spellings deserve a comment. `struct Register<T>` declares the parameter; `impl<T: Copy> Register<T>` declares it *again* for the methods, and that is where the bound lives — `read()` hands back a copy of the value, so the methods require `T: Copy`, which every integer satisfies. Keeping the struct definition unbounded and putting requirements on the `impl` is idiomatic: the data doesn't care what `T` can do; the *operations* do.

And note what the type system now knows: `Register<u8>` and `Register<u16>` are **different types**, exactly as SystemVerilog's `register #(byte)` and `register #(shortint)` specializations were. Try to write a `u16` result into the 8-bit A register and the program does not compile — where SV, having kept the widths straight in the declaration, would have quietly truncated at the assignment, and Python kept widths straight by discipline and masking alone. Here the widths are in the types and the seams are sealed. This — a struct parameterized by the type it carries — is precisely what `Vec<T>` has been all along, and it is the shape rustdv's plumbing takes: when Chapter 31 connects components with typed FIFOs, a FIFO of ALU commands is a `TlmFifo<AluCommand>`, and connecting it to a port expecting results is a compile error, not a 3 a.m. discovery.

## Monomorphization, or: where did the ducks go?

Now the question a performance-minded verification engineer should be asking. Python's duck typing has a runtime cost, and it is not small: *every* method call, every attribute access — `item.compare(other)`, `self.scoreboard.write(txn)` — is a lookup by name, at runtime, every single time, because until the moment of the call Python does not know what the object is. Twenty million transactions means twenty million rounds of "does this object have a `write`?" That is the interpreter overhead Chapter 1 put on the ledger.

So what does `largest` cost? It is one function that works on three types — surely *something*, somewhere, is checking which type showed up?

Nothing is, and the reason is the best idea in this chapter. At compile time, the compiler finds every concrete type your program actually uses with `largest`, and generates a separate, specialized copy of the function for each one — a process called **monomorphization**, "making single-formed." Your generic source code is a stencil; the compiler stamps it out once per type. Conceptually, figure 2 compiles as if you had written:

```rust
// Figure 5: What the compiler generates from figure 2 (conceptually — you never see this)

fn largest_u8(list: &[u8]) -> &u8 {
    // ... same body, with T = u8 throughout
}

fn largest_u16(list: &[u16]) -> &u16 {
    // ... same body, with T = u16 throughout
}

fn largest_str(list: &[&str]) -> &&str {
    // ... same body, with T = &str throughout
}
```

Each call site in `main` is wired directly to its own stamped-out copy. When `largest(&operands)` runs, there is no lookup, no vtable, no "which type is this?" — the `u8` version was chosen before the program existed as a binary, and the call is exactly as fast as the hand-written `u8`-only function you refused to copy-paste three times. The same happens to structs: `Register<u8>` and `Register<u16>` compile to two independent struct definitions, each with its own specialized methods. SystemVerilog engineers have seen this movie — each parameterization of an SV class is its own specialization too; Rust's version simply happens before any simulator gets involved. This is what the design of rustdv means by *generic code costs nothing at runtime*: you write the abstraction once and pay for it never.²

Set the two models side by side, because this is the chapter's picture. Python answers "which `write()` do I call?" by looking it up when the call happens — maximally flexible, paid for on every transaction of every test of every regression. Rust answers the same question once, at compile time, by generating the exact code each caller needs — and the flexibility you give up is precisely the flexibility of being wrong. Chapter 10's trait objects (`Box<dyn Animal>`, the vtable, the runtime dispatch) remain available for the cases that need runtime choice; monomorphized generics are what you get everywhere else, which in a testbench is almost everywhere.

Honesty requires the other side of the ledger, and it is real but modest: stamping out copies takes compile time and makes the binary larger. A generic function used with thirty types is compiled thirty times. For testbench code — a handful of transaction types, not thirty — you will notice this approximately never, but now you know why heavily generic Rust code compiles slower than it runs.

## The destination: SeqItemPort<REQ, RSP>

Part I keeps promising that these language chapters are load-bearing, so let me show you the load — and give SystemVerilog its due, because SV-UVM got this design *right*: `uvm_driver #(type REQ = uvm_sequence_item, type RSP = REQ)` is a parameterized class, and typing the driver's conversation was the correct instinct. pyuvm, living in dynamic Python, dropped the parameters — what came out of `get_next_item()` was whatever the sequencer sent, and a misconfigured test discovered the mismatch at runtime, usually as an `AttributeError` deep in `run_phase()`.

rustdv keeps SystemVerilog's design and adds Rust's enforcement. Signatures only — Chapters 31 and 36 build this for real:

```rust
// Figure 6: The shape of rustdv's driver (preview — signatures only)

pub struct SeqItemPort<REQ, RSP = REQ> { /* channel endpoints — elided */ }

pub struct AluDriver {                       // your driver: a plain struct...
    seq_item_port: SeqItemPort<AluCommand>,  // ...that owns a typed port
    // ...
}
```

Everything in this chapter is in the first line. Two type parameters, because a driver's conversation has two directions: `REQ` is the request transaction it pulls from the sequencer, `RSP` the response it may send back. Type parameters can have *defaults* — `RSP = REQ` says "if you don't name a response type, it's the same as the request," the exact convention `uvm_driver`'s declaration has carried for two decades — so the common no-response port is just `SeqItemPort<AluCommand>`. There is no `uvm_driver` base class to extend — your driver is an ordinary struct that owns a port — but the port, the sequencer on its far end, and every transaction that crosses between them must agree on the types *or the testbench does not compile*. The wrong-sequence-on-the-wrong-agent bug does not become an error message. It becomes unwritable.

That is the trade this book keeps making, in its purest form yet: runtime flexibility — any port, any transaction, sort it out mid-simulation — exchanged for a signature that is documentation, contract, and proof all at once. It is the design SV-UVM sketched with parameterized classes, finished. And thanks to monomorphization, `SeqItemPort<AluCommand>` compiles to code as direct as a port hand-written for ALU commands alone, because that is literally what the compiler makes of it.

## Summary

Generics let one definition serve many types: type parameters in angle brackets stand for a type named later, on functions (`fn largest<T: PartialOrd>(list: &[T]) -> &T`) and on structs (`Register<T>`, and the `Vec<T>`, `Option<T>`, and `Result<T, E>` you have used since Chapter 8). Trait bounds are the contract — `T: PartialOrd`, or several requirements joined with `+`, or a `where` clause when the list grows — and they finish what SystemVerilog's parameterized classes started while replacing Python's duck typing with the same idea checked at compile time: if it implements the bound, it's a duck, and the compiler verifies the feathers before the program runs. Monomorphization is why none of this costs anything at runtime: the compiler stamps out a specialized copy of the generic code for each concrete type used, so every call dispatches directly, in exchange for some compile time and binary size. And `SeqItemPort<REQ, RSP = REQ>` is where the book is taking all of it — a driver whose port carries its transaction types in its type, so mismatched testbench plumbing fails at compile time instead of mid-regression.

We now have types that carry proofs. What we do not yet have is Rust's way of passing *behavior* around — the thing Python did every time it handed a function to `sorted(key=...)` or stored a coroutine to run later, and the thing rustdv's factory will do for a living. Chapter 12 takes up closures and iterators, and if you enjoyed watching the compiler specialize your generics for free, you are going to like what it does to a `for` loop.

> ¹ When inference can't decide — or you want to be explicit — you can name the type at the call site with `largest::<u8>(&operands)`. The `::<>` operator is universally called the *turbofish*, it is official Rust culture, and no, nobody has come up with a better name. Swim on.

> ² C++ programmers will recognize this as what templates do — and SystemVerilog's parameterized classes inherited the same per-instantiation behavior — with one upgrade worth appreciating: a Rust generic function is type-checked once, against its bounds, when it is *defined*, not re-checked per instantiation with errors erupting from the template's guts. The compiler in figure 1 complained about our signature, in our terms, before any caller existed.
