//! Chapter 29: The Factory.
//!
//!     sim-common/run_sim.sh ch29_factory playground
//!
//! No DUT: the subject is *which type gets built*, not what it drives.
//!
//! rustdv has a factory, and it works the way the UVM factory works: you
//! build a component through it, and code above you can substitute a
//! different type without touching the code that built it. From where you
//! sit it is the same tool you already know (D74) — the differences are all
//! under the floor:
//!
//! - Registration is **universal and automatic**: `#[derive(Component)]`
//!   enrols every component by name, so any of them can be created by name
//!   or overridden, with no "did I remember to register it?" (D73). Same as
//!   pyuvm's metaclass and SV's `uvm_component_utils`.
//! - A component is overridable only if you build it with `create()`. Build
//!   it with `default()` and it is fixed. That choice — factory or not — is
//!   how a block author decides what a reuser may swap (D69).
//! - Two constructors, the UVM/pyuvm pair transcribed: `Foo::new_comp()` is
//!   the normal way (fixed, like UVM's `new`), `Foo::create_comp()` goes
//!   through the factory (overridable, like UVM's `create`). Neither takes
//!   arguments — the name and parent come from the tree (D7), not from you.
//! - A child field is always **`RustdvComp`** — a slot that holds any
//!   component. It says nothing about position in the tree (any component
//!   can be a top here and a child there) and nothing about overridability.
//!   Overridability is decided when you *fill* the slot, not by its type:
//!   `new_comp()` is fixed, `create_comp()` is overridable. Same field type
//!   either way; the build line is the control (D69). The framework can see
//!   the difference by looking — a `create_comp()` slot is flagged, and the
//!   walk checks it for an override and swaps it if one applies.
//! - There is no separate factory object full of proxies. A "maker" is an
//!   ordinary Rust value, so the override table is just the configuration
//!   store from Chapters 27–28. You never see that; you call `Factory::…`.
//!
//! Port of the Python book's chapter 33.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Creating a component through the factory
// ===========================================================================

// Chapter 29, Figure 1: A tiny example component.
//
// `#[derive(Component)]` registers it — universally, like every other
// component — so it can be created by type *or* by the string "TinyComponent"
// and can be the target of an override. Nothing else is needed.
#[derive(Component, Default)]
struct TinyComponent;

impl Component for TinyComponent {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("tiny");
        ctx.info("I'm so tiny!");
        Ok(())
    }
}

// Chapter 29, Figure 2: Building the component the normal way.
//
// `new_comp()` is rustdv's plain constructor — the analogue of UVM's `new`.
// No name, no parent; both come from the tree (D7). A component built this
// way is **not** overridable: nobody upstream can swap it, because it never
// went through the factory (D69).
//
// The struct is identical to Figure 4's — same `RustdvComp` field. The only
// difference is this one line: `new_comp()` versus `create_comp()`.
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

// Chapter 29, Figure 4: Building the component through the factory.
//
// `create_comp()` takes no arguments either — the same shape as `new_comp()`.
// With no override in force it builds a `TinyComponent`, so this logs exactly
// what Figure 2 did. The difference is invisible here and total later: this
// one *can* be overridden, by type or by instance, and you never named it.
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

// Chapter 29, Figure 6: Building a component from a string name.
//
// The name is data — here a literal, in Chapter 39 a line from a file. The
// universal registry (Figure 1's derive) is what turns the string back into
// a constructor. An unregistered name is a testbench bug and panics; a name
// read from a file will instead return a `Result` (Chapter 39).
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

// ===========================================================================
// Overriding a type
// ===========================================================================

// Chapter 29, Figure 7: The component we will substitute in.
#[derive(Component, Default)]
struct MediumComponent;

impl Component for MediumComponent {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("medium");
        ctx.info("I'm medium size.");
        Ok(())
    }
}

// Chapter 29, Figure 8: Overriding TinyComponent with MediumComponent, by type.
//
// The env still writes `TinyComponent::create_comp()` — it is not edited and
// does not know it was overridden. The test, above it, installs the override
// first. Because build is top-down, the test's build runs before the walk
// reaches the component and checks its flag, so the override is in force by
// the time it matters.
//
// The type pair is checked at compile time: `MediumComponent` must be a
// component (a `Box<dyn ComponentNode>` maker is generated from it), so an
// override with a non-component will not build. SV catches the analogue at
// run time with `$cast`; the check is earlier here, though the bug it catches
// is a simple one.
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

// Chapter 29, Figure 10: The same override, by string name.
//
// When the type isn't a compile-time name — it came from a file, or a plugin
// — you override by string. This cannot be compile-checked (the names are
// data), so a name that is not registered fails at this call, the way SV's
// `$cast` would.
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

// ===========================================================================
// Overriding one instance
// ===========================================================================

// Chapter 29, Figure 12: An environment with two components of the same type.
//
// Both children are built exactly the same way — `TinyComponent::create_comp()`,
// no name. To override just *one* of them, the factory has to tell them
// apart, and it can: the fields are named `tc1` and `tc2`, and when the walk
// reaches each field it checks that slot's override against the path it
// lands at. You never type "tc1" here; the struct already said it.
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

// Chapter 29, Figure 13: Overriding only env.tc1.
//
// The path picks the instance. `tc2`, at a different path, is untouched — so
// this test prints one "medium size" and one "so tiny". The path *is* a
// string, but it is naming a component elsewhere in the tree, exactly as
// `ConfigDb::set` addresses a remote path (Chapter 27) — it does not
// duplicate a name the field already carries.
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

// ===========================================================================
// Debugging the factory
// ===========================================================================

// Chapter 29, Figure 15: Printing the overrides in force.
//
// A resolved build tells you what got built; the override listing tells you
// *why*. It is the same store the ConfigDb dumps (Chapter 28), shown through
// the factory's window — you do not need to know that to read it.
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
