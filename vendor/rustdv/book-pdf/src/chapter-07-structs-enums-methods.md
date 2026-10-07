# Chapter 7: Structs, Enums, and Methods

Both earlier books in this series built on the same claim: object-oriented programming is the foundation of the expandability and reusability of UVM-based verification. That claim survives the trip to Rust intact. What does not survive is the machinery. Rust has no `class` statement, no `self` or `this` sliding invisibly into every method, no `__init__`, and no ability to bolt a data attribute onto an object whenever the mood strikes. In their place it offers two building blocks — structs and enums — and a place to hang behavior on them, the `impl` block. By the end of this chapter you will have both, plus the chapter's real payoff: Rust enums, which are so much more capable than Python's or SystemVerilog's that they will quietly restructure how you think about testbench data.

> **In the UVM...** every transaction and component was a class. Python let us build objects on the fly — `walrus = Animal()` then `walrus.kg = 1000`, no declaration anywhere — a freedom that cut both ways, surfacing a forgotten attribute as a runtime `AttributeError`. SystemVerilog made us declare every field at definition, as this chapter will too. And both dialects named the TinyALU's operations with an enum — `typedef enum` in SV, `Ops(enum.IntEnum)` in Python: `ADD = 1, AND = 2, XOR = 3, MUL = 4`, names mapped to opcode integers. Both patterns return in this chapter, and both come back changed.

## Structs: the data, declared

A Rust struct is the part of a class that holds data — and only that part. You declare every field, with its type, up front:

```rust
// Figure 1: Defining and instantiating a struct

struct Animal {
    kg: f64,
}

fn main() {
    let walrus = Animal { kg: 1000.0 };
    println!("Walrus mass: {}", walrus.kg);
}
```

```text
--
Walrus mass: 1000
```

Two things deserve a look. First, the declaration reads like SystemVerilog, not Python: the fields are part of the definition, not something we improvise later. Second, instantiation uses a *struct literal* — `Animal { kg: 1000.0 }` — which names every field and supplies every value. There is no separate "create empty, then populate" step, because an empty `Animal` is not a thing Rust permits to exist.

That last point is not a style preference; it is enforcement. In Python you could create a `yorkie`, forget to set `kg`, and get an `AttributeError` — at runtime, at the moment of use, which in a testbench means mid-simulation. Here is the same mistake in Rust:

```rust
// Figure 2: Forgetting a field is now a compile error

struct Animal {
    kg: f64,
}

fn main() {
    let yorkie = Animal {};
    println!("Yorkie mass: {}", yorkie.kg);
}
```

```text
--
error[E0063]: missing field `kg` in initializer of `Animal`
 --> src/main.rs:6:18
  |
6 |     let yorkie = Animal {};
  |                  ^^^^^^ missing `kg`
```

The program never ran. There is no such thing as an `Animal` with an undefined mass, so the class of bug Python could only warn about is not a bug you can write.¹ SystemVerilog engineers will feel at home — and should note the upgrade: an SV field you never assign holds a default value and simulates anyway; a Rust field you never assign stops the build. This is the pattern of the whole chapter — of the whole book, really: where your old language offered a convention and a warning, Rust offers a rule and a compile error.

One freedom is gone: you cannot add a field to an object after the fact. `walrus.age = 12` on a struct with no `age` field does not compile. Every field a transaction will ever carry is visible in one place, in its definition, forever. For quick scripts this feels confining. For a transaction type that five components and three engineers share, it is exactly what you want.

> ¹ The walrus, having survived one book already, takes this in stride.

## `impl` blocks: where behavior lives

Python and SystemVerilog classes bundle data and behavior in one block. Rust separates them: the struct declares the data, and an `impl` block — short for *implementation* — attaches the behavior. Here is a `get_pounds()` method:

```rust
// Figure 3: A method in an impl block

struct Animal {
    kg: f64,
}

impl Animal {
    fn get_pounds(&self) -> f64 {
        self.kg / 2.2
    }
}

fn main() {
    let walrus = Animal { kg: 1000.0 };
    println!("Walrus weight in pounds {:.2}", walrus.get_pounds());
}
```

```text
--
Walrus weight in pounds 454.55
```

The call site reads like every language you know: `walrus.get_pounds()`. The definition is where the languages diverge, and the divergence is instructive.

Python's `self` is an ordinary argument that the interpreter fills in for you — `Animal.get_pounds(walrus)` works, passing the object by hand. (SystemVerilog's `this` is closer to true magic; it simply appears.) Rust's `&self` is doing the same job, but it carries more information: that ampersand says this method *borrows* the animal, read-only, exactly as Chapter 6 taught. The method can look at `self.kg`; it cannot modify it, and it does not take ownership of the walrus. The signature tells you the method's relationship to the object before you read a line of the body.

And yes, the explicit form still works: `Animal::get_pounds(&walrus)` is legal Rust and does exactly what Python's hand-passed version does. What Python offered as a party trick, Rust treats as the ordinary meaning that the dot syntax abbreviates.

When a method needs to *change* the object, it says so:

```rust
// Figure 4: A method that mutates takes &mut self

struct Animal {
    kg: f64,
}

impl Animal {
    fn feed(&mut self, meal_kg: f64) {
        self.kg += meal_kg;
    }
}

fn main() {
    let mut walrus = Animal { kg: 1000.0 };
    walrus.feed(3.5);
    println!("Walrus mass after lunch: {}", walrus.kg);
}
```

```text
--
Walrus mass after lunch: 1003.5
```

Notice `let mut walrus`. A method that takes `&mut self` can only be called on a mutable binding — immutability by default, as in Chapter 3, extends all the way into method calls. Skim any `impl` block and the `&self`/`&mut self` split hands you a map of which methods observe and which methods mutate. In your old languages, discovering that a method quietly modified your transaction was an afternoon with the debugger; here it is a fact printed in the signature. When we build monitors and scoreboards, this distinction stops being philosophy: a scoreboard's check wants `&self`, its update wants `&mut self`, and the compiler holds every caller to it.

## Associated functions: `new()` and the end of `__init__`

Python forces initialization with `__init__()`, a magic method called implicitly when you invoke the class name; SystemVerilog answers to the `new` keyword and a `function new()` you write. Rust has no magic methods and no implicit calls. What it has is the *associated function*: a function in an `impl` block that takes no `self` at all, called through the type name with `::`.

By near-universal convention, the constructor is an associated function named `new` — SystemVerilog engineers may enjoy that Rust agrees with them about the name:

```rust
// Figure 5: The new() associated function

struct Animal {
    kg: f64,
}

impl Animal {
    fn new(kg: f64) -> Self {
        Self { kg }
    }

    fn get_pounds(&self) -> f64 {
        self.kg / 2.2
    }
}

fn main() {
    let yorkie = Animal::new(20.0);
    println!("The Yorkie weighs {:.1} pounds", yorkie.get_pounds());
}
```

```text
--
The Yorkie weighs 9.1 pounds
```

A few notes on the pattern:

- `Self` (capital S) is shorthand for "the type this `impl` block belongs to" — here, `Animal`. Writing `-> Self` and `Self { kg }` means the code survives a rename.
- `Self { kg }` uses *field init shorthand*: when a variable and a field share a name, you may write the name once instead of `kg: kg`.
- There is nothing special about `new`. It is not a keyword, the language does not call it for you, and you could name it `hatch()` if you enjoyed confusing people. The compiler's only contribution is the guarantee from figure 2: however an `Animal` gets built, every field gets a value.

Python sorts methods into three kinds — instance, class, and static, a decorator apiece — and SystemVerilog splits its own hairs with `static` methods and variables. Rust flattens the taxonomy to two: if it takes `self` in some form, it is a *method*, called with a dot; if it does not, it is an *associated function*, called with `::`. The static and class-method patterns both collapse into the second kind, and the class-variable pattern comes along as an *associated constant*:

```rust
// Figure 6: Associated constants and functions replace class variables and static methods

struct Triangle;

impl Triangle {
    const SIDE_COUNT: u32 = 3;

    fn print_side_count() {
        println!("I have {} sides.", Self::SIDE_COUNT);
    }
}

fn main() {
    Triangle::print_side_count();
}
```

```text
--
I have 3 sides.
```

(`struct Triangle;` with no braces is a *unit struct* — a type with no data, which is all our triangle ever needed.) The two classic reasons to keep a static method inside a class — a logical home, and a consistent calling style — apply verbatim here; the `impl` block *is* the logical home, and `Triangle::print_side_count()` is the consistent style.

## Enums: the star of the chapter

Now for the construct that earns this chapter its place in the book.

Python names the TinyALU's operations with an `enum.IntEnum` — named constants, each secretly an integer wearing a name tag. SystemVerilog's `typedef enum` is the same idea with the same secret: underneath, it is an integer, and the language happily casts it back to one at the first opportunity. Rust enums start at that point and keep going.

Here is `Ops`, together with the prediction function it exists to serve:

```rust
// Figure 7: The Ops enumeration and an exhaustive match

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
    match op {
        Ops::Add => a as u16 + b as u16,
        Ops::And => (a & b) as u16,
        Ops::Xor => (a ^ b) as u16,
        Ops::Mul => a as u16 * b as u16,
    }
}

fn main() {
    let op = Ops::Add;
    println!("{:?} of 0x0A and 0x05 -> 0x{:04x}", op, alu_prediction(0x0A, 0x05, op));
}
```

```text
--
Add of 0x0A and 0x05 -> 0x000f
```

Working through the new pieces:

**The `derive` line.** `#[derive(Clone, Copy, Debug, PartialEq)]` asks the compiler to generate standard capabilities: copying, comparison with `==`, and the `{:?}` debug printing you see in `main`. These are *traits*, and Chapter 10 gives them their due; for now, read the line as "make this type behave like a sensible value." IntEnum gave Python's `Ops` comparison and printing for free by inheritance; the derive line is where Rust's version of "for free" lives.

**The discriminants.** `Add = 1` assigns the variant an integer value, just as the IntEnum and the `typedef enum` did, and `Ops::Add as u8` recovers it when the opcode has to go onto a bus. But note which way the equivalence runs. An IntEnum member *is* an int — you can add three to `Ops.ADD` and Python will let you — and an SV enum converts to an integer whenever the expression around it shrugs. `Ops::Add` is not a number that happens to have a name; it is a value of type `Ops`, and arithmetic on it is a compile error. The integer is available on request, one direction only. (Going the other way — from a raw integer read off a bus back to an `Ops` — takes explicit code that must confront the possibility of an illegal opcode. Chapter 9's `Result` is built for exactly that confrontation.)

**The `match`.** Chapter 4 introduced `match` as the load-bearing construct; here is the load. A `match` on an enum must be *exhaustive*: every variant handled, or the code does not compile. The prediction function cannot silently do nothing for `Mul` the way a Python `if/elif` chain with no `else` can.

That guarantee sounds abstract until the day it saves you. Suppose the TinyALU grows a subtract instruction. Add `SUB = 5` to the IntEnum or the `typedef enum`, and every `if/elif` chain and every `case` over ops in the testbench — the prediction function, the coverage model, the driver — keeps running, silently wrong, until a failing test (or worse, a passing one) sends you hunting. Watch what happens in Rust the moment we add the variant and change nothing else:

```rust
// Figure 8: The compiler finds every match the new variant breaks

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
    Sub = 5,   // the new operation — and the only edit we made
}
```

```text
--
error[E0004]: non-exhaustive patterns: `Ops::Sub` not covered
  --> src/main.rs:11:11
   |
11 |     match op {
   |           ^^ pattern `Ops::Sub` not covered
   |
note: `Ops` defined here
   = note: the matched value is of type `Ops`
help: ensure that all possible cases are being handled by
      adding a match arm with an explicit pattern
```

We changed the specification — one line — and the compiler produced a complete list of every place in the testbench that has not yet heard the news, before any simulator license was checked out, before any simulation ran, before any test could pass for the wrong reason. In a runtime-checked flow, this bug costs a simulation run at minimum and a shipped escape at maximum. Here it costs one compile, a few seconds, and it *cannot* be skipped.²

> ² There is an escape hatch — a `_ => ...` wildcard arm matches everything not yet named, and using one forfeits this protection. The idiom this book follows: never use a wildcard when matching an enum you own. Spend the extra lines; they are the tripwire.

## Four states, no integers: `Logic`

SystemVerilog carries `x` and `z` natively — that is what `logic` is for. Python does not: every value in a cocotb testbench was ultimately an integer, and `get_int()` existed precisely to coerce simulator values that might contain `x` or `z` into something Python could compute with. Rust sides with SystemVerilog — hardware signals are not integers — and lets us say so directly:

```rust
// Figure 9: A four-state Logic enum

#[derive(Clone, Copy, Debug, PartialEq)]
enum Logic {
    Zero,
    One,
    X,
    Z,
}

fn to_char(v: Logic) -> char {
    match v {
        Logic::Zero => '0',
        Logic::One => '1',
        Logic::X => 'x',
        Logic::Z => 'z',
    }
}

fn main() {
    let bit = Logic::X;
    println!("The signal reads: {}", to_char(bit));
}
```

```text
--
The signal reads: x
```

`Logic` is not a teaching toy: when we reach rustdv-sim in Chapter 17, reading a signal hands you a `LogicArray` — a vector of exactly this type — ported from the same four-state value types cocotb defines. Notice what the enum buys us that an IntEnum encoding (say, `X = 2`) never could — and that even SystemVerilog's native `logic` does not: there is no integer pretense to leak. In SV, an `x` rides along in arithmetic as x-propagation and surfaces downstream as a mystery; here, nothing can accidentally add `X` to a running sum, because `X` is not a number — it is one of four states a wire can be in, and any code that consumes a `Logic` must, thanks to exhaustive `match`, say what it does about `x` and `z`. The "forgot to handle the unknown state" bug is unrepresentable.

## Variants that carry data

Everything so far, an IntEnum could at least gesture at. This last capability it could not. Rust enum variants can *carry data* — different data per variant — which makes an enum a type that says "this value is exactly one of the following shapes." Computer scientists call this a *sum type*; testbench authors will call it the right way to model outcomes:

```rust
// Figure 10: An enum whose variants carry payloads

#[derive(Debug)]
enum CheckResult {
    Pass,
    Fail { expected: u16, actual: u16 },
}

fn check(expected: u16, actual: u16) -> CheckResult {
    if expected == actual {
        CheckResult::Pass
    } else {
        CheckResult::Fail { expected, actual }
    }
}

fn main() {
    let result = check(0x000f, 0x0005);
    match result {
        CheckResult::Pass => println!("PASS"),
        CheckResult::Fail { expected, actual } => {
            println!("FAIL: expected 0x{expected:04x}, got 0x{actual:04x}")
        }
    }
}
```

```text
--
FAIL: expected 0x000f, got 0x0005
```

Read the `match` arms closely, because a new thing is happening: the second arm doesn't just select the `Fail` variant, it *unpacks* it, binding `expected` and `actual` right there in the pattern. A passing check carries nothing; a failing check carries the evidence. In your old languages this was a boolean plus a couple of variables that are only meaningful when the boolean is false — a convention, enforced by hope. The enum makes the convention structural: the payload *exists* only in the variant it belongs to, and the only way to touch it is to admit, in a pattern, which case you are in.

Once you see this shape, you will see it everywhere in Rust, because the standard library's two most important types are exactly this pattern: `Option<T>` is an enum whose variants are `Some(T)` and `None`, and `Result<T, E>` is an enum whose variants are `Ok(T)` and `Err(E)`. Absence and failure, modeled as payload-carrying enums, matched exhaustively — that is the entire story of how Rust replaced both `None`-checking and exceptions, and it gets Chapter 9 to itself.

## Summary

Rust splits the class into parts and makes each part explicit. A **struct** declares its data completely — every field, every type, no after-the-fact attributes — and the compiler guarantees no instance ever exists with a field unset, retiring Python's `AttributeError` surprise and SystemVerilog's silently-defaulted field alike. An **`impl` block** attaches behavior: **methods** take `&self` to observe or `&mut self` to mutate, telling every caller which is which, while **associated functions** take no `self` and answer to the type name — `Animal::new(20.0)` — absorbing constructors, class methods, and static methods with one mechanism and zero magic.

**Enums** are the chapter's prize. `Ops` returns not as named integers but as a true sum type: exhaustively matched, so that adding a variant produces a compiler-generated to-do list of every site that must change — a testbench bug class eliminated before simulation. `Logic` models a wire's four states without pretending they are numbers. And payload-carrying variants let one type hold differently-shaped alternatives — the pattern behind `Option` and `Result`, and behind more testbench types to come.

Our transactions can now hold data and our enumerations can now hold their own. What we cannot yet do is hold *many* of anything — a queue of commands, a list of results, a scoreboard's worth of expectations. Chapter 8 takes up Rust's collections, where the Python sequences you know are waiting, with ownership along for the ride.
