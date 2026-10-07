# Chapter 21: Macros: Code That Writes Code

Every `#[rustdv::test]` you have typed since Chapter 15 has been an IOU. This chapter pays it. The chapters ahead lean on two pieces of compile-time machinery — the test attribute and `#[derive(Component)]` — and you deserve to know exactly what replaced the machinery you left behind: SystemVerilog's `` `uvm_component_utils `` macros, pyuvm's decorators and metaclasses. That is a language question with a twist, because Python's tools ran at *import time* with the full power of a running program, SystemVerilog's ran in the preprocessor as text substitution, and Rust has neither an import time nor a text preprocessor.

> **In the UVM...** the framework organized our code for us, each dialect in its own way. SystemVerilog used macros: `` `uvm_component_utils(my_driver) `` expanded — textually, before the compiler proper ever saw it — into the factory-registration boilerplate nobody wanted to hand-write. Python used runtime hooks: `@cocotb.test()` received our coroutine as an argument, registered it in a global list, and handed it back, and pyuvm went further, with a *metaclass* registering every component class with the factory as a side effect of the `class` statement itself.

## What a decorator actually did

Strip `@cocotb.test()` to its skeleton and it is ten lines of Python:

```python
# Figure 1: What @cocotb.test() really does — a registering decorator

test_registry = []

def test():
    def wrapper(coro):
        test_registry.append(coro)   # side effect at import time
        return coro
    return wrapper

@test()
def hello_world():
    print("Hello, world.")

@test()
def wait_2ns():
    print("I am DONE waiting!")

print(f"registered {len(test_registry)} tests:")
for t in test_registry:
    print(f"  {t.__name__}")
```

```text
--
registered 2 tests:
  hello_world
  wait_2ns
```

The decorator is an ordinary function that runs *when the module is imported*, mutating a global list. That is the whole trick, and it is a good trick — cocotb's regression manager later walks `test_registry` (in real life, with options and filters) and runs what it finds. The trick's precondition is the part Rust lacks: **a moment when code runs because source code was loaded.** Rust programs are compiled, then executed; nothing happens in between, and `use` (Chapter 14) triggers nothing. So Rust splits the decorator's two jobs — *transforming code* and *registering it* — into two different mechanisms, and this chapter takes them in turn.

## Declarative macros: patterns in, code out

Rust's entry-level macro is `macro_rules!` — the family `println!`, `format!`, `assert_eq!`, and `vec!` come from, and the reason they carry an exclamation point: the `!` warns you that *arguments are being rewritten, not passed*. A declarative macro is a set of match arms over source-code patterns:

```rust
// Figure 2: A declarative macro — patterns in, code out

macro_rules! check {
    ($actual:expr, $expected:expr) => {
        if $actual == $expected {
            println!("PASSED: {} = {:04x}", stringify!($actual), $actual);
        } else {
            println!(
                "FAILED: {} = {:04x} - predicted {:04x}",
                stringify!($actual),
                $actual,
                $expected
            );
        }
    };
}

fn main() {
    let result: u16 = 0xFF + 0x01;
    check!(result, 0x0100);
    check!(result, 0x0000);
}
```

```text
--
PASSED: result = 0100
FAILED: result = 0100 - predicted 0000
```

`$actual:expr` binds any expression; the right side is the code that replaces the call, with the bindings spliced in. Notice what no function could do: `stringify!($actual)` prints the *source text* of the argument — the reason `assert_eq!` failures can show you the expression that failed, and something Python decorators managed only by introspecting live objects. Declarative macros are pure rewriting, hygienic, and resolved entirely inside the compiler. rustdv uses them sparingly — `first!`, `join!`, `vpi_bootstrap!` — and the book's advice is the community's: reach for a macro only after a function or generic has failed you.

## Procedural macros: the compiler takes arguments

The heavier tool — and the true decorator replacement — is the **procedural macro**: a Rust function that runs *inside the compiler*, receives your code as a stream of tokens, and returns the token stream to compile instead. Where a decorator transformed a live function object at import time, a proc macro transforms source code at compile time. They come in the two flavors this book cares about: **attribute macros** (`#[rustdv::test]`), which replace the item they decorate, and **derive macros** (`#[derive(Clone)]`, `#[derive(Component)]`), which read a type's definition and append new code alongside it.

So: what does `#[rustdv::test]` actually emit? Here is the expansion, lightly tidied, for `async fn hello_world(...)`:

```rust
// Figure 3: What #[rustdv::test] expands to (tidied)

// 1. Your function, untouched — the attribute adds, never rewrites:
async fn hello_world(_ctx: RustdvCtx) -> Result<(), TestError> { /* your body */ }

// 2. A shim with a uniform signature, so the runner can hold
//    every test in one list (Box<dyn Future>, Chapter 13):
fn __rustdv_shim(ctx: RustdvCtx) -> Pin<Box<dyn Future<Output = Result<(), TestError>>>> {
    Box::pin(hello_world(ctx))
}

// 3. The registration, contributed to a distributed slice:
#[linkme::distributed_slice(TEST_REGISTRATIONS)]
static __RUSTDV_TEST_REG: TestRegistration = TestRegistration {
    name: "hello_world",
    module: module_path!(),
    file: file!(),
    line: line!(),
    run: __rustdv_shim,
    timeout: None,
    skip: false,
    expect_fail: false,
};
```

Parts 1 and 2 are the decorator's *wrapping* job, done with types: your `async fn` stays exactly as you wrote it, and the shim adapts it to the one shape the regression runner stores. The attribute's arguments — `timeout_time`, `timeout_unit`, `expect_fail`, `skip`, `name` — are cocotb's `Test` options, parsed by `syn` into Rust syntax and emitted with `quote`; a typo'd option is a compile error pointing at the attribute. `file!()` and `line!()` capture the source location the runner prints in `running hello_world (1/2) [ch15-.../src/ch15_async_await_executor.rs:12]` — you have been reading this macro's output in every transcript since Chapter 15.

Part 3 is the *registering* job, and it needs its own section, because there is no global list and no import time to fill one.

## Registration without a runtime: the linker as registry

The trick is old, standard, and delightful: **let the linker build the array.** The `linkme` crate packages it as a *distributed slice*: declare the slice once, then contribute static values from any file or crate in the build. `linkme` places those values into the platform's linker sections and presents the result as an ordinary shared slice. In miniature, and runnable:

```rust
// Figure 4: Link-time registration in miniature

use linkme::distributed_slice;

/// What a registration carries: a name and a function to run.
struct Registration {
    name: &'static str,
    run: fn(),
}

#[distributed_slice]
static TESTS: [Registration];

fn hello() {
    println!("Hello, world.");
}
#[distributed_slice(TESTS)]
static REG_HELLO: Registration = Registration { name: "hello", run: hello };

fn goodbye() {
    println!("Goodbye, world.");
}
#[distributed_slice(TESTS)]
static REG_GOODBYE: Registration = Registration { name: "goodbye", run: goodbye };

fn collect() -> &'static [Registration] {
    &TESTS
}

fn main() {
    let tests = collect();
    println!("found {} registered tests:", tests.len());
    for t in tests {
        print!("  {} -> ", t.name);
        (t.run)();
    }
}
```

```text
--
found 2 registered tests:
  goodbye -> Goodbye, world.
  hello -> Hello, world.
```

Read the output closely: `goodbye` came out *first*. Link order is the linker's business, not yours — which is why the real rustdv runner sorts its collected registrations by `(file, line)` before running, so a regression's order is the order tests appear in your source. `linkme` owns the platform-specific section names, retention attributes, boundary symbols and pointer work; rustdv iterates the resulting slice through safe Rust.¹

This is the moment to bank a comparison the rest of the book builds on. cocotb discovers tests because importing your module *runs* registration code. rustdv discovers tests because compiling your crate *emits* registrations into the binary. Both are "the framework finds your tests by name" — same user experience, and command-line selection by test name works the same way — but the Rust version happens entirely before execution, cannot be affected by import order, and works in a `cdylib` the simulator loads, where "import time" would be a meaningless phrase.

> ¹ The technique has platform texture — ELF section symbols on Linux differ from Mach-O and Windows spellings — which is precisely why rustdv delegates it to `linkme` instead of maintaining those spellings itself.

## Derive macros: field lists instead of `__dict__`

The other half of Python's runtime magic was introspection: pyuvm's `do_copy` and `do_compare` walked `self.__dict__` at runtime to copy and compare whatever fields a transaction happened to have, and its factory metaclass registered classes by watching them be defined. Rust cannot look up a struct's fields at runtime — the names are gone by then — but a **derive macro** can look at them at *compile time*, which is how `#[derive(Clone, Debug, PartialEq)]` has been writing your `do_copy`, `convert2string`, and `do_compare` since Chapter 10: the derive receives the struct definition as tokens, iterates the fields, and emits field-by-field implementations. Same field-walking idea as `__dict__`, run once, in the compiler, with the results type-checked.

rustdv ships exactly one derive of its own, and the rest of the book uses it everywhere: `#[derive(Component)]`. Given a component struct, it reads the fields you mark as children and writes the boilerplate that a UVM-ish framework needs to walk your hierarchy:

```rust
// Figure 5: What #[derive(Component)] writes for you (tidied)

#[derive(rustdv::Component)]
pub struct AluEnv {
    seqr: Sequencer<AluCommand>,
    #[component]
    driver: Option<Driver>,
    #[component]
    scoreboard: Scoreboard,
}

// ...expands to (hand-writable, if you ever prefer):
impl ComponentNode for AluEnv {
    fn node_name(&self) -> &'static str { "AluEnv" }
    fn visit_children(&mut self, f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {
        if let Some(__c) = &mut self.driver { f("driver", __c); }
        f("scoreboard", &mut self.scoreboard);
    }
}
```

Three things to notice, all previews of Chapter 24. The hierarchy's names come from your *field names* — `driver`, `scoreboard` — synthesized at compile time, where pyuvm passed name strings to every constructor. `Option<Driver>` is understood: a `None` child (the passive agent's missing driver) is simply skipped, and `Vec<T>` children get indexed names like `drivers[0]`. And the tidied expansion above shows the traversal only — the full one *also registers the component by name*, contributing to a second `linkme` distributed slice. That is pyuvm's factory metaclass, kept: every component can be created by name or overridden with no separate registration step, and Chapter 29 collects on it. Both metaclass jobs — test discovery and component registration — turned out to be the linker's.

## When not to write a macro

A chapter that hands you power tools owes you the safety lecture. Macro-generated code is code you didn't write and can't click into; error messages inside a macro expansion point at generated text; and every macro is a small language your teammates must learn. The bar this book applies — and applied to rustdv itself — is: a macro must delete user-visible boilerplate *and* be explainable in one paragraph. `#[rustdv::test]` clears the bar (it deletes a shim and a registration you should never hand-write). `#[derive(Component)]` clears it (field-walking is the machine's job — as true here as it was for the `uvm_field_*` macros and pyuvm's `__dict__` walk). Everything else in rustdv — the lifecycle, the ConfigDb, TLM, sequences — is plain code, on purpose, so that when you read the UVM chapters you are reading Rust, not incantations.

## Summary

Python organized testbenches at import time: decorators wrapped and registered functions; metaclasses registered classes; `__dict__` introspection copied and compared objects. Rust has no import time, and this chapter met its replacements. Declarative macros (`macro_rules!`) rewrite patterns into code — the `!` family you have used all book. Procedural macros run inside the compiler: the `#[rustdv::test]` attribute leaves your function intact and emits a uniform shim plus a registration; `#[derive(...)]` reads field lists at compile time and writes the member-wise code pyuvm wrote by runtime reflection, including rustdv's own `#[derive(Component)]` — hierarchy traversal plus by-name registration. Registration without a runtime rides in the linker: `linkme` gathers distributed statics into slices, and rustdv sorts tests by source location — import-order bugs structurally impossible. And the governing taste: macros where dynamism used to be, plain code everywhere else.

That closes the language's account, and it was the last debt outstanding. You now know how tests are found and how hierarchies will be walked and registered. The chapters ahead can finally ask this series' biggest question in its new language: what is the UVM *for*? Chapter 22: Why UVM?
