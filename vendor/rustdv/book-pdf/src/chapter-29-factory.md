# Chapter 29: The Factory

The UVM factory answers a question every test writer eventually asks: *how do I change what the testbench does without editing the testbench?* One environment, closed and finished, should serve many tests — and configuration alone only changes *values*. To change what a slot in the hierarchy is *built as*, you need construction itself to be interceptable. That is the factory, and rustdv has one, working the way the factory you know works: build a component through it, and code above you can substitute a different type without touching the code that built it.

> **In the UVM...** we instantiated components through the factory — `tiny_component::type_id::create("tc", this)` in SystemVerilog, `TinyComponent.create("tc", self)` in pyuvm — instead of calling the constructor; then `set_type_override_by_type(...)` made every subsequent create of a Tiny produce a Medium, and an instance override targeted one path. Registration happened behind our backs — the `` `uvm_component_utils `` macro in SV, a metaclass at import time in Python — and `factory.print()` listed the overrides in force.

Everything in that box has a direct rustdv counterpart, and this chapter walks them in the same order the Python book's factory chapter does. The differences are under the floor, and the chapter will point at each as it goes by.

## Creating a component through the factory

```rust
// Chapter 29, Figure 1: A tiny example component
#[derive(Component, Default)]
struct TinyComponent;

impl Component for TinyComponent {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("tiny");
        ctx.info("I'm so tiny!");
        Ok(())
    }
}
```

Nothing here mentions the factory, and that is the first difference worth noticing: **registration is universal and automatic.** `#[derive(Component)]` enrolls every component by name, so `TinyComponent` can be created by type or by the string `"TinyComponent"`, and can be the target of an override, with no separate registration step and no "did I remember the utils macro?" This is the same promise pyuvm's metaclass makes, kept by the derive you were already writing. 

Now, two ways to build one:

```rust
// Chapter 29, Figure 2: Building the component the normal way
#[rustdv::test]
#[derive(Component, Default)]
struct TinyTest {
    #[component]
    tc: RustdvComp,
}

impl Component for TinyTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tc = TinyComponent::new_comp();
    }
}
```

```text
# Figure 3: The normal way builds what it says

      0.00ns INFO     running TinyTest (1/7)  [ch29-factory/src/ch29_factory.rs:71]
      0.00ns INFO     [TinyTest.tc]: I'm so tiny!
```

`new_comp()` is rustdv's plain constructor — the analog of UVM's `new`. Note what it does *not* take: no name, no parent. Both come from the tree, as they have since Chapter 24. A component built this way is fixed; nobody upstream can swap it, because it never went through the factory.

```rust
// Chapter 29, Figure 4: Building the component through the factory
#[rustdv::test]
#[derive(Component, Default)]
struct TinyFactoryTest {
    #[component]
    tc: RustdvComp,
}

impl Component for TinyFactoryTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tc = TinyComponent::create_comp();
    }
}
```

```text
# Figure 5: The factory way builds the same thing — until someone objects

      0.00ns INFO     running TinyFactoryTest (2/7)  [ch29-factory/src/ch29_factory.rs:90]
      0.00ns INFO     [TinyFactoryTest.tc]: I'm so tiny!
```

Put figures 2 and 4 side by side: identical structs, one `RustdvComp` field each, and exactly one line different — `new_comp()` versus `create_comp()`. With no override in force they even log the same output. The difference is invisible here and total later: `create_comp()` flags the slot, and the build walk checks flagged slots for an override and swaps in the substitute if one is installed. This is the `new` versus `create` distinction every UVM engineer already carries, transcribed — and it puts a real decision in the block author's hands: **overridability is the build line, not the field type.** A `RustdvComp` field says nothing about whether its occupant can be swapped; the line that fills it says everything. Write `create_comp()` for the slots a reuser may replace, `new_comp()` for the ones they may not.

The string form completes the set:

```rust
// Chapter 29, Figure 6: Building a component from a string name
#[rustdv::test]
#[derive(Component, Default)]
struct CreateByNameTest {
    #[component]
    tc: RustdvComp,
}

impl Component for CreateByNameTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tc = Factory::create_by_name("TinyComponent");
    }
}
```

The name is data — here a literal, in a bigger testbench a line from a command file. The universal registry is what turns the string back into a constructor, and this test logs exactly what figures 3 and 5 did. An unregistered name is a testbench bug and fails at this call — names are data, and no compiler checks data.

## Overriding a type

```rust
// Chapter 29, Figure 7: The component we substitute in
#[derive(Component, Default)]
struct MediumComponent;

impl Component for MediumComponent {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("medium");
        ctx.info("I'm medium size.");
        Ok(())
    }
}
```

```rust
// Chapter 29, Figure 8: Overriding TinyComponent with MediumComponent, by type
#[rustdv::test]
#[derive(Component, Default)]
struct MediumFactoryTest {
    #[component]
    tc: RustdvComp,
}

impl Component for MediumFactoryTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        Factory::set_type_override::<TinyComponent, MediumComponent>();
        self.tc = TinyComponent::create_comp();
    }
}
```

```text
# Figure 9: The same create line builds something else

      0.00ns INFO     running MediumFactoryTest (4/7)  [ch29-factory/src/ch29_factory.rs:151]
      0.00ns INFO     [MediumFactoryTest.tc]: I'm medium size.
```

Read the build closely, because its two lines are doing Chapter 24's argument one more time. The create line is unedited — it still says `TinyComponent::create_comp()`, and in a real testbench it would live in an environment that never learns it was overridden. The override is installed *above*, before the create runs, and it is build's top-down direction that guarantees the ordering: the test's `build` runs before the walk descends to the slot, so the override is in force by the time it matters. This is the gap between existing and having children, doing exactly the job Chapter 24 promised the factory would need it for.

One check does happen at compile time, and it is worth being exact about which: the type pair. `set_type_override::<TinyComponent, MediumComponent>()` requires the substitute to *be* a component — the maker generated from it must produce a tree node — so overriding with a non-component does not build. SystemVerilog catches that analog at run time in `$cast`. Which slot gets overridden, though, and whether anyone creates a `TinyComponent` at all — those resolve at run time, by design, because deferring them is what the factory is *for*.

When even the types are data, the override is too:

```rust
// Chapter 29, Figure 10: The same override, by string name
#[rustdv::test]
#[derive(Component, Default)]
struct MediumNameTest {
    #[component]
    tc: RustdvComp,
}

impl Component for MediumNameTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        Factory::set_type_override_by_name("TinyComponent", "MediumComponent");
        self.tc = TinyComponent::create_comp();
    }
}
```

```text
# Figure 11: Same substitution, by name

      0.00ns INFO     running MediumNameTest (5/7)  [ch29-factory/src/ch29_factory.rs:171]
      0.00ns INFO     [MediumNameTest.tc]: I'm medium size.
```

## Overriding one instance

A type override hits every flagged slot that asks for the type. Sometimes you want just one:

```rust
// Chapter 29, Figure 12: An environment with two components of the same type
#[derive(Component, Default)]
struct TwoCompEnv {
    #[component]
    tc1: RustdvComp,
    #[component]
    tc2: RustdvComp,
}

impl Component for TwoCompEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tc1 = TinyComponent::create_comp();
        self.tc2 = TinyComponent::create_comp();
    }
}
```

```rust
// Chapter 29, Figure 13: Overriding only env.tc1
#[rustdv::test]
#[derive(Component, Default)]
struct TwoCompTest {
    #[component]
    env: Option<TwoCompEnv>,
}

impl Component for TwoCompTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        Factory::set_inst_override::<TinyComponent, MediumComponent>(ctx, "env.tc1");
        self.env = Some(TwoCompEnv::default());
    }
}
```

```text
# Figure 14: The path picks the instance

      0.00ns INFO     running TwoCompTest (6/7)  [ch29-factory/src/ch29_factory.rs:218]
      0.00ns INFO     [TwoCompTest.env.tc1]: I'm medium size.
      0.00ns INFO     [TwoCompTest.env.tc2]: I'm so tiny!
```

Notice what the env did *not* do: it built `tc1` and `tc2` with two identical `create_comp()` calls, no names typed. The factory can still tell them apart because the *fields* are named — when the walk reaches each slot, it checks the override against the path it landed at, and `env.tc1` matches while `env.tc2` does not. The path in `set_inst_override` is a string, but it is a string doing the same job `ConfigDb::set`'s path does: addressing a component elsewhere in the tree, from a place that has no other way to point at it. It does not duplicate a name the field already carries.

(And the demonstration is not vacuous: delete the `set_inst_override` line and both children log "I'm so tiny!"; restore it and only `tc1` changes. The override drives the outcome — a rerun anyone can do.)

## Debugging the factory

Chapter 28 gave the ConfigDb a dump because a resolved value doesn't show the competition. The factory has the same need — a resolved build tells you what got built, not *why* — and the same answer:

```rust
// Chapter 29, Figure 15: Printing the overrides in force
#[rustdv::test]
#[derive(Component, Default)]
struct PrintOverridesTest {
    #[component]
    tc: RustdvComp,
}

impl Component for PrintOverridesTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        Factory::set_type_override_by_name("TinyComponent", "MediumComponent");
        self.tc = TinyComponent::create_comp();
    }

    fn end_of_elaboration(&mut self, _ctx: &mut RustdvCtx) {
        Factory::print();
    }
}
```

```text
# Figure 16: The overrides in force, listed

      0.00ns INFO     running PrintOverridesTest (7/7)  [ch29-factory/src/ch29_factory.rs:241]
      0.00ns INFO     Factory overrides:
      0.00ns INFO       *                           : TinyComponent -> MediumComponent
      0.00ns INFO     [PrintOverridesTest.tc]: I'm medium size.
```

`Factory::print()` at `end_of_elaboration`, for Chapter 28's reason: the hierarchy is final, nothing has run, and what you see is what every `create_comp()` resolved against. (Under the floor it is the same store the ConfigDb dumps, seen through the factory's window — a fact you can enjoy and never need.)

## Summary

The factory, ported whole and working as the one you know: `new_comp()` is `new` and fixed, `create_comp()` is `create` and overridable, and the choice between them is the block author deciding what a reuser may swap — per build line, not per type. Registration is universal via the derive, so create-by-name and override-by-name need no bookkeeping; type overrides check at compile time only that the substitute is a component, and everything else — which slots, which paths, whether the override fires at all — resolves during the top-down build walk, which is the moment Chapter 24's gap exists to provide. Instance overrides tell twins apart by the paths their field names created, and `Factory::print()` shows the standing orders when a build surprises you.

Testbench 5.0 puts the factory to work: one environment with a variation point, and two tests that fill it differently without touching a line of the env.
