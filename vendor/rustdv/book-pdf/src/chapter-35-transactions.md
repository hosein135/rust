# Chapter 35: Transactions

The 6.0 testbench passes `(u8, u8, Ops)` tuples, and everyone is tired of remembering that `op` is the thing at index 2. The UVM's answer was `uvm_object`: named-field transaction classes with standard copy, compare, and print machinery. This chapter gives the TinyALU its real transactions, by way of the same warm-up the Python book used — a person with an ID, then a student with a list of grades — because the grades list is what makes copying *visible*, and three scalars cannot show it.

> **In the UVM...** we extended `uvm_sequence_item` and got the machinery of `uvm_object`: `clone()` backed by a `do_copy()` that walked the fields — with the "always call `super().do_copy(other)` first" discipline; equality backed by `do_compare()`; printing via `convert2string()` (`__str__()` in pyuvm), overridden by hand for every class; plus the long tail — pack, unpack, record — that the specification demands and most testbenches quietly ignore.

Before the first listing, one adjustment of mental furniture, because it decides how every figure below reads. **A rustdv transaction is not an object. It is a location in memory, referred to by a name.** `AluCommand { a, b, op }` is a layout — three fields side by side — and `cmd` is a binding to that place. Nothing is wrapped around it, nothing points at it, and it has no identity separate from its bytes. That is why there is no base class in this chapter: there is no object for a base class to be part of. And it is why there is no `super()`: no inheritance chain means no chain to copy up. What the base class *gave* you — printing, comparing, copying — arrives instead as traits attached from the outside, which is why they are derives rather than inherited methods. You have known this since Chapter 7; this chapter is the reminder aimed at the UVM habit, which will otherwise keep looking for the object.

## Two string forms

```rust
// Figure 1: A transaction is a plain struct with two string forms

#[derive(Debug)]
struct PersonRecord {
    name: String,
    id_number: u32,
}

// `Display` is `convert2string()` / `__str__()`. It is **not** derivable —
// you write it, because only you know which fields are worth reading.
impl fmt::Display for PersonRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {}", self.name, self.id_number)
    }
}

fn main() {
    let xx = PersonRecord { name: String::from("Joe Shmoe"), id_number: 37 };

    // `{}` asks for Display: the form you chose.
    println!("Printing the record: {xx}");
    println!("Logging the record:  {}", xx.to_string());

    // `{:?}` asks for Debug: every field, mechanically, for free.
    println!("Debug form:          {xx:?}");
}
```

```text
--
Printing the record: Name: Joe Shmoe, ID: 37
Logging the record:  Name: Joe Shmoe, ID: 37
Debug form:          PersonRecord { name: "Joe Shmoe", id_number: 37 }
```

Rust gives a transaction *two* string forms, and they are different jobs. `Debug` (`{:?}`) is the developer field-dump: every field, mechanically formatted, generated free by the derive — the thing you want when debugging the testbench. `Display` (`{}`) is `convert2string()`: the readable form, and it is *not derivable* — you write it, because only the author knows that a command reads best as operand-operator-operand. Reach for `Display` in log messages and `Debug` in despair, and do not confuse the two: the derive cannot write your `convert2string()` for you, and does not try.

## Equality is a decision

```rust
// Figure 2: You decide what "the same" means

// `#[derive(PartialEq)]` compares **every field**.
#[derive(Debug, PartialEq)]
struct StrictRecord {
    name: String,
    id_number: u32,
}

// But "the same" is a decision, not a fact. Here two records are the same
// person when the ID matches, whatever the name says.
#[derive(Debug)]
struct PersonRecord {
    name: String,
    id_number: u32,
}

impl PartialEq for PersonRecord {
    fn eq(&self, other: &PersonRecord) -> bool {
        self.id_number == other.id_number
    }
}

// ... (the Display impl from Figure 1, unchanged)

fn main() {
    println!("-- derived: every field must match --");
    let a = StrictRecord { name: String::from("Batman"), id_number: 27 };
    let b = StrictRecord { name: String::from("Bruce Wayne"), id_number: 27 };
    println!("batman == bruce_wayne? {}", a == b);

    println!("-- written by hand: only the ID counts --");
    let batman = PersonRecord { name: String::from("Batman"), id_number: 27 };
    let bruce_wayne = PersonRecord { name: String::from("Bruce Wayne"), id_number: 27 };
    println!("batman == bruce_wayne? {}", batman == bruce_wayne);

    if batman == bruce_wayne {
        println!("Batman is really Bruce Wayne!");
    }
}
```

```text
--
-- derived: every field must match --
batman == bruce_wayne? false
-- written by hand: only the ID counts --
batman == bruce_wayne? true
Batman is really Bruce Wayne!
```

`PartialEq` is `do_compare()`, and it lands in the same two flavors the UVM gives it. Derived, it compares every field — the undemanding default, usually right for a transaction. Written by hand, it encodes a *policy*: these two records are the same person because only the ID counts, whatever the name field says. The UVM makes you write `do_compare()` for exactly these cases, and rustdv puts the policy in the same place the UVM does — **on the transaction**. Equality is the data type's own statement about itself, not something a scoreboard improvises per comparison.

## Why equality comes in two traits

```rust
// Figure 3: Why equality comes in two traits

// `PartialEq` promises symmetry and transitivity. It does **not** promise
// that a == a. That sounds like hair-splitting until you meet a float:
// IEEE 754 says NaN is equal to nothing, including itself.
#[derive(Debug, PartialEq)]
// #[derive(Eq)]  // <-- will not compile: f64 is not Eq
struct Measurement {
    delay_ns: f64,
}

// `Eq` adds the missing promise — every value equals itself — and carries no
// methods. `HashSet` and `HashMap` require it, because a key that does not
// equal itself could never be found again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

fn main() {
    let m = Measurement { delay_ns: f64::NAN };
    println!("a measured delay is not equal to itself: NaN == NaN? {}", m == m);

    // `Ops` is `Eq`, so it can be a coverage key.
    let mut seen: HashSet<Ops> = HashSet::new();
    for op in [Ops::Add, Ops::Mul, Ops::And, Ops::Xor, Ops::Add] {
        seen.insert(op); // the last one is already in the set
    }
    println!("ops seen: {}", seen.len());
}
```

```text
--
a measured delay is not equal to itself: NaN == NaN? false
ops seen: 4
```

New material, with a verification-shaped bite. Rust splits equality into two traits because `PartialEq` does not promise `a == a` — IEEE 754 forbids it for NaN — while `Eq` adds that promise and nothing else. Where this catches a verification engineer is the coverage bin: `HashSet` requires `Eq`, so a transaction carrying a *measured* value — a float delay, a sampled analog level — cannot be a coverage key, and the compiler says so at the derive. Uncomment the `Eq` on `Measurement` and the error names `f64` as the reason. The ops enum, all integers underneath, derives `Eq` and `Hash` and has been serving as a coverage key since Chapter 18.

## Copying: deep for what you own

```rust
// Figure 4: Clone is deep for what you own

// A student is a person who also owns a list of grades. The `Vec` is the
// interesting part: it is the field that would make a shallow copy visible.
#[derive(Debug, Clone)]
struct StudentRecord {
    name: String,
    id_number: u32,
    grades: Vec<u32>,
}

impl fmt::Display for StudentRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {} Grades: {:?}", self.name, self.id_number, self.grades)
    }
}

fn main() {
    let mary = StudentRecord {
        name: String::from("Mary"),
        id_number: 33,
        grades: vec![97, 82],
    };

    let mut mary_copy = mary.clone();

    println!("mary:      {mary}");
    println!("mary_copy: {mary_copy}");

    println!("-- add a grade to the copy --");
    mary_copy.grades.push(100);

    println!("mary:      {mary}");
    println!("mary_copy: {mary_copy}");
}
```

```text
--
mary:      Name: Mary, ID: 33 Grades: [97, 82]
mary_copy: Name: Mary, ID: 33 Grades: [97, 82]
-- add a grade to the copy --
mary:      Name: Mary, ID: 33 Grades: [97, 82]
mary_copy: Name: Mary, ID: 33 Grades: [97, 82, 100]
```

`Clone` is `do_copy()`, generated from the field list, and cloning a field you *own* clones its contents: `mary_copy` got its own `Vec`, not a second name for Mary's, and the pushed grade proves it. Python needs both `copy.copy()` and `copy.deepcopy()` because assignment shares references and shallow is the accident you start from. Rust has one `clone()`, and how deep it goes is already written *in the type*: owned data is copied. There is no way for figure 4 to have gone the other way — the compiler would not let two records share a `Vec` without your saying so. Which brings us to how you say so:

```rust
// Figure 5: If you want the shallow copy, you ask for it

// Same record, one field changed: the grades now live behind an `Rc`, so a
// clone copies the *handle* and both records point at one list.
#[derive(Debug, Clone)]
struct SharedStudent {
    name: String,
    grades: Rc<RefCell<Vec<u32>>>,
}

fn main() {
    let mary = SharedStudent {
        name: String::from("Mary"),
        grades: Rc::new(RefCell::new(vec![97, 82])),
    };

    let mary_copy = mary.clone();

    println!(
        "before: {} {:?} / {} {:?}",
        mary.name, mary.grades.borrow(), mary_copy.name, mary_copy.grades.borrow()
    );

    println!("-- add a grade through the copy --");
    mary_copy.grades.borrow_mut().push(100);

    println!(
        "after:  {} {:?} / {} {:?}",
        mary.name, mary.grades.borrow(), mary_copy.name, mary_copy.grades.borrow()
    );

    println!("handles to the one list: {}", Rc::strong_count(&mary.grades));
}
```

```text
--
before: Mary [97, 82] / Mary [97, 82]
-- add a grade through the copy --
after:  Mary [97, 82, 100] / Mary [97, 82, 100]
handles to the one list: 2
```

This is Python's `copy.copy()` — except that in Python sharing is what you get by default, and here you had to write `Rc` to ask for it, with `RefCell` along because shared-and-mutable moves the borrow check to run time (Chapter 13). The deep/shallow decision is settled once, in the type definition, where a reviewer can see it — not at every call site, where the Python book had to warn you about it.

## copy() and clone(), by their own names

```rust
// Figure 6: clone_from is the UVM's copy(): fill an object you already have

#[derive(Debug, Clone, Default)]
struct StudentRecord {
    name: String,
    id_number: u32,
    grades: Vec<u32>,
}

impl fmt::Display for StudentRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, ID: {} Grades: {:?}", self.name, self.id_number, self.grades)
    }
}

fn main() {
    let mary = StudentRecord {
        name: String::from("Mary"),
        id_number: 33,
        grades: vec![97, 82],
    };

    // uvm_object.copy(other)  ->  Clone::clone_from(&mut self, source)
    // uvm_object.clone()      ->  Clone::clone(&self)
    let mut mary_copy = StudentRecord::default();
    println!("before: {mary_copy}");

    mary_copy.clone_from(&mary);
    println!("after:  {mary_copy}");

    println!("-- and clone() is clone(): a new one, returned --");
    let fresh = mary.clone();
    println!("fresh:  {fresh}");
}
```

```text
--
before: Name: , ID: 0 Grades: []
after:  Name: Mary, ID: 33 Grades: [97, 82]
-- and clone() is clone(): a new one, returned --
fresh:  Name: Mary, ID: 33 Grades: [97, 82]
```

The UVM has two copying calls and Rust has the same two, nearly under the same names: `copy(other)` fills an object you already have, and so does `clone_from`; `clone()` hands back a new one in both worlds. What has no counterpart is `do_copy()` itself, and the discipline that came with it — "always call `super().do_copy(other)` first," which the UVM needs because a copy must walk up an inheritance chain. The derive walks the field list instead; there is no first step to forget.

One false friend before the payoff, because the words collide: Rust's `Copy` trait is unrelated to the UVM's `copy()`. `Copy` means "duplicating this is a memcpy and the original stays usable" — it is a statement about ownership, not about transactions, and a transaction owning a `String` or a `Vec` cannot be `Copy` at all. Chapter 31 showed what goes wrong when a `Copy` stand-in lets the wrong retry loop compile.

## The TinyALU transactions, final form

```rust
// Figure 7: The TinyALU transactions, final form

// `Eq` and `Hash` because coverage puts an op in a `HashSet` (Figure 3).
// `Copy` because an op is one byte and copying it is free — the one place in
// the testbench where `Copy` is the right answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

impl Ops {
    pub const ALL: [Ops; 4] = [Ops::Add, Ops::And, Ops::Xor, Ops::Mul];
}

// A command is plain data: no base class, no framework trait, nothing to
// extend. Not `Copy`: a transaction is something you hand over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

// `Display` is the one you write.
impl fmt::Display for AluCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A: {:02x} {:?} B: {:02x}", self.a, self.op, self.b)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AluResult {
    pub result: u16,
}

fn main() {
    let cmd = AluCommand { a: 0xA5, b: 0x75, op: Ops::Mul };

    println!("cmd:        {cmd}");
    println!("debug:      {cmd:?}");

    let copy = cmd.clone();
    println!("clone == cmd? {}", copy == cmd);

    let mut tweaked = cmd.clone();
    tweaked.a = 0;
    println!("tweaked == cmd? {}", tweaked == cmd);

    let mut covered: HashSet<Ops> = HashSet::new();
    for op in Ops::ALL {
        covered.insert(op);
    }
    println!("ops covered: {} of {}", covered.len(), Ops::ALL.len());

    let _ = AluResult { result: 0x4B69 };
}
```

```text
--
cmd:        A: a5 Mul B: 75
debug:      AluCommand { a: 165, b: 117, op: Mul }
clone == cmd? true
tweaked == cmd? false
ops covered: 4 of 4
```

Everything the chapter covered, applied. These three definitions replace the tuples the testbench has carried since Chapter 19, and they serve every remaining chapter of the book: `Ops` is `Copy` because a one-byte op is the testbench's one honest `Copy` case, and `Eq + Hash` because coverage keys demand it; `AluCommand` is *not* `Copy`, because a transaction is something you hand over — the fact `finish_item` will lean on in the next chapter — and its derived `Clone`, `PartialEq`, and `Debug` are `do_copy`, `do_compare`, and the field dump, each regenerating itself whenever you add a field. The one method written by hand is `Display`, because it always was the one method that needed an author.

## Summary

A transaction is a place, not an object: no base class to extend, no identity beyond its bytes, no `super()` chain to remember. The `uvm_object` services arrive as traits — `Debug` free from the derive, `Display` written by hand because `convert2string()` always was, `PartialEq` derived when every field counts and hand-written when "the same" is a policy, `Clone` deep for owned fields with sharing spelled `Rc` in the type, and `clone_from`/`clone` matching `copy()`/`clone()` name for name. `Eq` is the extra Rust asks so a value can promise it equals itself, and coverage keys are where you will feel it.

Because a transaction is a place and not a handle, handing one to `finish_item` will hand over its *contents* — and the sequence testbench that runs on that rule is next.
