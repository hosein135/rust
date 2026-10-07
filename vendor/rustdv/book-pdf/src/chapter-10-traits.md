# Chapter 10: Traits

This chapter takes on the biggest piece of machinery yet: object-oriented programming itself. The UVM is built on inheritance in every language it speaks — `uvm_driver` extends `uvm_component`, which extends `uvm_object`, and every constructor dutifully calls its parent's, `super.new()` or `super().__init__()`, up the chain. Both earlier books spent multiple chapters on that machinery, because the methodology cannot run without it.

Rust has no inheritance. None. There is no base class, no child class, no `super`, no method resolution order to print. When I first learned this I assumed Rust must be missing something essential, and I was wrong in an instructive way: inheritance was never the *goal*. It was the mechanism every UVM language shared for two separate goals — sharing behavior across types, and treating different types uniformly. Rust delivers both with a single feature called a **trait**, and once you see the two goals pulled apart, you may find you don't miss the mechanism.

> **In the UVM...** we used inheritance to avoid copying code: a base class holds the shared behavior, a child extends it — `class lion extends animal` (SV) or `class Lion(Animal)` (Python) — and overrides what differs. A `virtual` method lets the child's override win even through a base-class handle, and every constructor's first duty is to invoke its parent's — `super.new(name, parent)`, `super().__init__(name, parent)` — a discipline the UVM enforces by breaking at runtime when you forget.

This chapter rebuilds that material piece by piece: the animal menagerie, the `super` question, multiple inheritance, and then the payoff for verification — how traits replace the machinery every transaction class has always needed: `do_compare()` and `convert2string()` in SystemVerilog, `__eq__` and `__str__` in Python.

## Shared behavior without a base class

A trait is a named list of method signatures that a type can opt into. It declares *what* a type can do; a separate `impl` block declares that a particular type does it. Teaching OOP with an animal menagerie is a tradition in this series — the Primer's lion said Roar, the Python book's dog said bow bow — and Rust will not be the book that breaks it.

```rust
// Figure 1: Shared behavior through a trait, not a base class

trait Animal {
    fn species(&self) -> &str;
    fn sound(&self) -> &str;

    fn make_sound(&self) {
        println!("The {} says '{}'", self.species(), self.sound());
    }
}

struct Dog;
struct Cat;

impl Animal for Dog {
    fn species(&self) -> &str { "dog" }
    fn sound(&self) -> &str { "bow bow" }
}

impl Animal for Cat {
    fn species(&self) -> &str { "cat" }
    fn sound(&self) -> &str { "miāo" }
}

fn main() {
    Dog.make_sound();
    Cat.make_sound();
}
```

```text
--
The dog says 'bow bow'
The cat says 'miāo'
```

Read the trait from the bottom up. `species()` and `sound()` are *required* methods — they have signatures but no bodies, so any type implementing `Animal` must provide them. If "a method with a signature but no body, which every child must provide" sounds familiar, it should: it is SystemVerilog's `pure virtual function` in an abstract class, except that a trait's requirements are checked wherever the `impl` is compiled, not when a class finally dares to extend the abstract base. `make_sound()` has a body, which makes it a **default method**: implementors get it for free and may override it. The shared behavior a base class used to hold lives in the default method; the per-type differences the constructors used to hold live in each `impl` block.

Notice what is *not* here. `Dog` does not mention `Animal` in its definition — `struct Dog;` stands alone, and the relationship is declared separately, in `impl Animal for Dog`. `class Dog extends Animal` in SystemVerilog, like `class Dog(Animal)` in Python, fused "what Dog is" with "what Dog can do" into one statement. Rust keeps them apart, and the separation has a consequence you will come to rely on: you can implement a new trait for an existing type without touching the type's definition. When Chapter 32 needs a coverage collector to receive transactions, it will not edit the transaction — it will implement `Subscriber` on the collector, and the transaction never knows.

One more absence: data. A base class could declare a `species` field and let children fill it in. A trait *cannot contain fields* — only method signatures and default bodies. If shared behavior needs data, it asks for the data through a required method, exactly as `make_sound()` asks for `species()`. This will feel like ceremony for about one chapter, and then it will feel like honesty: the trait's signature documents precisely what it needs from you, instead of quietly reaching into your member variables and hoping.

## Where did super go?

Every UVM engineer carries a scar shaped like a forgotten `super` call. In SystemVerilog, a component whose `new()` skips `super.new(name, parent)` detaches itself from the component hierarchy and fails somewhere downstream, at runtime, with an error that mentions nothing about constructors. Python's version of the trap is sharper still — *Python for RTL Verification* built a figure around a `SmallDog` that extended `Dog`, overrode `__init__()`, forgot to call `super().__init__()`, and blew up with an `AttributeError`, because in Python data attributes are not inherited; they exist only if some constructor actually ran and created them. In both languages the fix is the same discipline: *always call up the chain*, and find out at runtime when you don't.

Rust closes this trap at both ends. First: struct fields are declared in the struct, not created by whichever initializer happens to run. There is no execution order that leaves a `SmallDog` half-built, because a struct that is missing a field does not compile. Second: when a `SmallDog` wants to reuse `Dog`'s behavior, it does so by *containing* a `Dog` — composition — and delegating to it explicitly.¹

```rust
// Figure 2: SmallDog by composition — delegation replaces super()

struct SmallDog {
    dog: Dog,    // a SmallDog HAS the dog parts
}

impl Animal for SmallDog {
    fn species(&self) -> &str { self.dog.species() }   // delegate to Dog
    fn sound(&self) -> &str { "yap yap" }              // override
}

fn main() {
    let sd = SmallDog { dog: Dog };
    sd.make_sound();
}
```

```text
--
The dog says 'yap yap'
```

Look at `self.dog.species()`. That line is doing the job every `super` call did, but spelled as what it actually is: a call to a specific method on a specific value. There is no method-resolution order to walk, no rule about which class comes next in the search, no way to forget the call and find out at runtime — if `SmallDog`'s `impl` doesn't provide `species()` one way or another, the compiler rejects the `impl` block on the spot, naming the missing method. The half-built object has no Rust equivalent to show you. The program that produces it cannot be built.

Python also offers `Dog.__init__(self)` — calling the parent's method explicitly through the class object — whose advantage is that you know exactly which copy you are calling. Delegation is that idea, promoted from alternative to only option. Rust decided that knowing which copy you are calling is not an advantage but a requirement.

## Wearing many hats

Here the two dialects part ways, and Rust sides with both of them at once. SystemVerilog forbids multiple inheritance outright — one parent per class, no exceptions — so an SV engineer has never had to ask which parent's constructor runs first. Python allows it, and *Python for RTL Verification* modeled Pat, a firefighter with kids, as `class FirefighterWithKids(Parent, Firefighter)`, followed by a careful discussion of the method resolution order and a design rule borrowed from *Highlander* — of `__init__()` methods, there can be only one.

In Rust, "Pat has two roles" is simply "Pat implements two traits" — SystemVerilog's restraint and Python's flexibility in the same feature. There is nothing to diagram.

```rust
// Figure 3: Multiple roles as multiple trait implementations

trait Parent {
    fn kiss(&self);
}

trait Firefighter {
    fn hose(&self);
}

struct Pat {
    name: String,
}

impl Parent for Pat {
    fn kiss(&self) { println!("{} gives the baby a kiss.", self.name); }
}

impl Firefighter for Pat {
    fn hose(&self) { println!("{} sprays water.", self.name); }
}

fn main() {
    let pat = Pat { name: String::from("Pat") };
    pat.kiss();
    pat.hose();
}
```

```text
--
Pat gives the baby a kiss.
Pat sprays water.
```

A type can implement as many traits as it likes, and because traits carry no data, the classic multiple-inheritance hazards — which parent's field wins, which constructor runs, what order the search visits the parents — never arise. `Pat` has exactly one set of fields, declared in exactly one place, and each trait bolts a capability onto it. The Highlander rule enforced itself.²

> ¹ The Python book credits the SmallDog change to Ron Swanson of Parks and Recreation. I see no reason to withdraw the attribution.

> ² Rust examined the method resolution order and politely declined to have one.

## Default methods: the UVM's no-op pattern, formalized

Here is where this chapter starts paying rent for Chapter 24. Think about how the UVM's phases work: `uvm_component` defines `build_phase()`, `connect_phase()`, `run_phase()` and the rest as *empty virtual methods*, and your components override only the phases they care about. The base class's no-op bodies exist so the phase machinery can call every phase on every component without checking what each component bothered to define.

That pattern — "here is the full interface; override what you use" — is exactly what default methods are for, and it is how rustdv's `Component` lifecycle trait is designed. A preview, signatures only (Chapter 24 does this properly):

```rust
// Figure 4: The shape of rustdv's Component trait (preview — signatures only)

pub trait Component {
    fn start(&mut self, ctx: &mut RustdvCtx) {}          // default: nothing to run
    fn check(&mut self, errors: &mut CheckSink) {}    // default: do nothing
    fn report(&self) {}                               // default: do nothing
}
```

A driver overrides `start()`; a scoreboard overrides `check()`; each inherits the empty default for every phase it ignores, and a purely structural component — an env that exists to hold its children — can implement the trait without overriding anything at all. The same division of labor the UVM has always used, with one upgrade. In the UVM, the empty methods live in a base class, so getting them requires joining the family tree rooted at `uvm_object`. In rustdv, they live in a trait you implement, so a component is just a plain struct — its fields are its children, per Chapter 24 — that opts into the lifecycle. Same methodology, no family tree.

## Deriving: the compiler writes the boring impls

Transactions need utility methods, and every UVM dialect makes you write them: equality so the scoreboard can compare a prediction to a result — `do_compare()` in SystemVerilog, `__eq__` in Python — and a string rendering so logs are readable — `convert2string()`, `__str__`. It has always been your job to write each one on every transaction class, field by field. Rust maps each of these jobs to a standard trait, and for most of them the compiler will write the implementation for you. The keyword is `#[derive(...)]`, an attribute you place on a struct or enum, listing traits whose implementations the compiler should generate from the fields.

Let's put the TinyALU's transaction under the microscope.

```rust
// Figure 5: Derived traits on the TinyALU command transaction

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
struct AluCommand {
    a: u8,
    b: u8,
    op: Ops,
}

fn main() {
    let cmd = AluCommand { a: 0xAA, b: 0x55, op: Ops::Xor };
    let copy = cmd.clone();
    println!("equal? {}", cmd == copy);
    println!("{:?}", cmd);
}
```

```text
--
equal? true
AluCommand { a: 170, b: 85, op: Xor }
```

One line above the struct bought us three implementations. `Clone` gives `cmd.clone()`, a field-wise copy — `do_copy()`, generated. `PartialEq` gives `==`, a field-wise comparison — this was you writing `do_compare()` or `__eq__` by hand, remembering to compare every field, and remembering again when you added a field. The derived version regenerates from the struct definition on every compile, so it *cannot* fall out of sync with the fields. `Debug` gives the `{:?}` format you see in the output: a machine-ish rendering of the whole struct, the job `sprint()` did for a `uvm_object` and `__repr__` did in Python.³

That leaves the human-friendly rendering — `convert2string()`, `__str__`. Its Rust counterpart is the `Display` trait, and here the compiler makes you write it yourself, on purpose: Rust's position is that a machine can guess how to *dump* your type but not how to *present* it. Implementing `Display` is our first hand-written implementation of a standard-library trait, and it looks like every trait impl you've seen this chapter:

```rust
// Figure 6: Implementing Display by hand — the __str__ of Rust

use std::fmt;

impl fmt::Display for AluCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cmd: a=0x{:02x} b=0x{:02x} op={:?}", self.a, self.b, self.op)
    }
}

fn main() {
    let cmd = AluCommand { a: 0xAA, b: 0x55, op: Ops::Xor };
    println!("{}", cmd);
}
```

```text
--
cmd: a=0xaa b=0x55 op=Xor
```

Once `Display` exists, `{}` in any format string works, exactly as `convert2string()` fed your `uvm_info` messages and `__str__` fed `print()`. The `write!` macro is `println!`'s cousin that writes into the formatter instead of to the screen, and the `fmt::Result` return is Chapter 9's `Result` making a cameo — formatting can fail, and the signature says so.

Here is the full mapping, the table this chapter exists to give you. Left columns: what your language made you write. Right columns: where it went.

| The job | SystemVerilog (`uvm_object`) | Python | Rust trait | How you get it |
|---|---|---|---|---|
| compare transactions | `do_compare()` | `__eq__` | `PartialEq` | `#[derive(PartialEq)]` |
| unambiguous debug dump | `sprint()` | `__repr__` | `Debug` (`{:?}`) | `#[derive(Debug)]` |
| human-readable string | `convert2string()` | `__str__` | `Display` (`{}`) | written by hand |
| ordering and sorting | — | `__lt__`, `__le__`, ... | `PartialOrd`, `Ord` | `#[derive(...)]` |
| hashing / keying | — | `__hash__` | `Hash` | `#[derive(Hash)]` |
| operator overloading | — | `__add__`, `__sub__`, ... | `std::ops::Add`, `Sub`, ... | written by hand |
| duplicate transactions | `do_copy()` | `copy.deepcopy()` | `Clone` | `#[derive(Clone)]` |

Three notes on the table. The SystemVerilog dashes are not gaps in the table but in the language — SV classes have no operator overloading and no standard ordering hook, which is why your scoreboards sort with hand-written comparison functions. `PartialOrd` and `Ord` split ordering in half because some types have values that refuse to be ordered (floating-point `NaN` is the culprit — hence *partial*); for transaction structs of integers and enums, you derive both and move on. And the operator traits in `std::ops` mean Rust has real operator overloading — `cmd_a + cmd_b` can be made to work — but it is opt-in per trait, per type.

If that table feels like it just dissolved a chapter of whichever book taught you these methods, hold the thought: Chapter 35 shows that it also dissolves most of `uvm_object`. The field-wise copy-and-compare machinery — which SV-UVM generates with `uvm_field_*` macros that walk the fields at runtime, and pyuvm hand-rolled by walking `__dict__` at runtime — is precisely what `derive` generates at compile time. A rustdv transaction is a plain struct with `#[derive(Clone, Debug, PartialEq)]` on top — no base class required.

> ³ The derived `Debug` prints `a: 170` rather than `a: 0xaa` because it renders a `u8` as decimal — another small argument for writing `Display` yourself when humans will read the result.

## Two kinds of polymorphism

We now have `Dog`, `Cat`, and `SmallDog`, each implementing `Animal`, and one question left — the question inheritance answered with "they share a base class": how do we write code that works on *any* animal?

Rust gives two answers, and choosing between them is a skill this book will exercise from here to the final chapter.

**Answer one: generics.** Write a function with a type parameter, and *bound* the parameter by the trait:

```rust
// Figure 7: Generic function — dispatch resolved at compile time

fn check_in<T: Animal>(animal: &T) {
    animal.make_sound();
}

fn main() {
    check_in(&Dog);
    check_in(&Cat);
}
```

```text
--
The dog says 'bow bow'
The cat says 'miāo'
```

`<T: Animal>` reads "for any type T that implements Animal." This is Python's duck typing with the ducks counted before the program runs: where Python said "just call `make_sound()` and hope," the bound *documents* the requirement in the signature and the compiler *checks* it at every call site. Behind the scenes the compiler compiles a separate copy of `check_in` for `Dog` and for `Cat` — a process called **monomorphization** — so each call dispatches directly, as fast as if you had written the two functions by hand. Generic code costs nothing at runtime. Chapter 11 is entirely about this machinery, so I'll leave it warm rather than cooked.

**Answer two: trait objects.** Sometimes you need one collection holding a mixture of types — a Python list held anything, and a kennel holds whatever shows up. For that, Rust erases the concrete type behind a pointer:

```rust
// Figure 8: Trait objects — dispatch resolved at runtime

fn main() {
    let kennel: Vec<Box<dyn Animal>> = vec![
        Box::new(Dog),
        Box::new(Cat),
        Box::new(SmallDog { dog: Dog }),
    ];
    for animal in &kennel {
        animal.make_sound();
    }
}
```

```text
--
The dog says 'bow bow'
The cat says 'miāo'
The dog says 'yap yap'
```

`dyn Animal` is a **trait object**: "some type, I'm not saying which, that implements Animal." Because the compiler no longer knows the concrete type, two things follow. The value must live behind a pointer — here Chapter 13's `Box` — since different animals have different sizes and a `Vec` needs uniform elements. And each call to `make_sound()` is dispatched at runtime through a **vtable**, a small table of function pointers riding along with the object. If that sounds like how Python method calls or SystemVerilog virtual methods work — yes, exactly, except that Rust makes you *ask* for dynamic dispatch by writing `dyn`, and hands you static dispatch everywhere else.

The trade, in one breath: generics are faster and fully checked but require the concrete types to be knowable where the code is compiled; trait objects accept types chosen at runtime but pay a pointer, an allocation, and an indirect call. The rule of thumb this book follows: **reach for generics first, and reserve `dyn` for collections that must mix types.**

You will see rustdv make both choices, each where it belongs. The driver is `Driver<REQ, RSP>` — generic over its transaction types, so handing the wrong transaction to a driver is a compile error rather than the `$cast` failure SV-UVM debugging is made of, or the runtime type explosion pyuvm checked for by hand. Subscribers are a bound, `T: Subscriber`, the "anything with a `write()` method" of the analysis chapters. And trait objects appear where heterogeneity is the point: the factory hands back every component it builds as a trait object, because a slot a test may override cannot commit to a concrete type (Chapter 29), and a sequencer stores its queued sequences as trait objects because sequences of different types wait in one line (Chapter 36). Known types: generics. One slot, many possible occupants: `dyn`.

## Summary

Rust replaces inheritance with traits: named, explicit interfaces that a type opts into with an `impl` block. Required methods state what the type must provide — pure virtual functions whose absence is caught at the `impl`, not at extension time; default methods carry shared behavior, doing the job of base-class methods — including the UVM's empty-phase-method pattern, which becomes default methods on rustdv's `Component` trait. Code reuse comes from composition and delegation, and the delegating call replaces `super` with an ordinary, visible method call. Multiple roles come from implementing multiple traits, with no diamond and nothing to resolve.

The transaction utility methods every UVM dialect demanded map one-for-one onto standard traits — `do_compare()`/`__eq__` to `PartialEq`, `convert2string()`/`__str__` to `Display`, `do_copy()` to `Clone`, debug dumps to `Debug` — and `#[derive(...)]` makes the compiler write most of them from your struct's fields, which is how a rustdv transaction gets by with no base class at all. Finally, "code that works on any implementor" comes in two flavors: generic functions with trait bounds, monomorphized and free at runtime, and trait objects behind `dyn`, dispatched through a vtable when one collection must hold many types.

We have been leaning on that `<T: Animal>` notation while promising the details later. Later has arrived — Chapter 11 opens up generics: type parameters, bounds, and why `Driver<REQ, RSP>` is the most honest thing a driver has ever said about itself.
