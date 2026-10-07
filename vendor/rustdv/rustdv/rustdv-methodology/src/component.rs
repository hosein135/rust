//! Component lifecycle and hierarchy traversal (design-doc §5.2/§5.3).
//!
//! > **Superseded, being replaced — do not build on it.** What this file
//! > still implements is review-memo R3: build/connect as *constructor
//! > conventions* rather than phases, a component's `new(config, ...)`
//! > constructing its children (`// build:`) and taking channel endpoints
//! > as arguments (`// connect:`), with only the runtime lifecycle left as
//! > a trait.
//! >
//! > **D5 and D6 reverse that, and the reversal has begun.** `build`
//! > (top-down) and `connect` (bottom-up) are real phase methods again
//! > (D51), because the gap between "a component exists" and "its children
//! > exist" is where all late binding lives — path-addressed configuration,
//! > factory overrides, TLM connection. The [`Component`] trait below now
//! > carries all nine phases and [`run_component_test`] drives them.
//! > `new(config, ...)`-style construction survives only in not-yet-
//! > converted testbenches (`tinyalu_tb`), not as the design.
//!
//! **What has landed.** Step 4 (D46–D49): a test is a component with an
//! `async fn run`, receiving the one universal [`RustdvCtx`]. Ch24 first
//! half (D51/D52): the nine phases, the phaser, path-aware phase logging.
//! Ch24 second half: two-stage construction (a parent creates children as
//! `Option<T>`/`Vec<T>` in its own `build`) and the bottom-up [`run_all`]
//! traversal firing every component's run.

use std::future::Future;
use std::pin::Pin;

use rustdv_sim::handle::HierarchyHandle;
use rustdv_sim::log::{Level, Logger};
use rustdv_sim::rng::Rng;

use crate::error::TestError;
use crate::objection::{ObjectionGuard, ObjectionRegistry};

type RunFuture<'a> = Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>>;

/// Agent activity (pyuvm's ConfigDB `is_active` int becomes an enum —
/// mapping row 42; illegal values are unrepresentable).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Active {
    Active,
    Passive,
}

/// Collector for `check`-phase failures (design-doc §5.3 signature).
#[derive(Default)]
pub struct CheckSink {
    errors: Vec<String>,
}

impl CheckSink {
    pub fn new() -> CheckSink {
        CheckSink::default()
    }
    pub fn error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        rustdv_sim::log::error(&msg);
        self.errors.push(msg);
    }
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
    pub fn into_result(self) -> Result<(), String> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} check failure(s): {}",
                self.errors.len(),
                self.errors.join("; ")
            ))
        }
    }
}

// ===========================================================================
// RustdvCtx — the one context (D47, which strikes D8)
// ===========================================================================

/// Everything a running testbench is handed: the DUT, randomization, the
/// objection registry, and the component's path.
///
/// **One type, not one per phase.** D8 wanted `BuildCtx`/`ConnectCtx`/
/// `RunCtx` so that `build_child` during run would fail to *compile*. D47
/// gives that up: Part II teaches a testbench with no components and
/// therefore no phases, and naming the type after a phase names a concept
/// the reader has not met. Phase-illegal operations are caught at run time,
/// as UVM catches them.
///
/// `Clone` is deliberate — the objection registry is `Rc`-shared, so a
/// clone objects to the same test. D9's per-node context, when the build
/// phase arrives, is that clone with the path extended.
#[derive(Clone)]
pub struct RustdvCtx {
    dut: HierarchyHandle,
    seed: u64,
    objections: ObjectionRegistry,
    logger: Logger,
}

impl RustdvCtx {
    /// A context with a path and no simulator, for unit tests.
    ///
    /// `dut()` will panic if called, which is the point: a test that reaches
    /// for the DUT needs a simulator and belongs in a `sim-*` case.
    #[cfg(test)]
    pub(crate) fn for_test(path: &str) -> RustdvCtx {
        RustdvCtx {
            dut: HierarchyHandle::null_for_test(),
            seed: 1,
            objections: ObjectionRegistry::new(),
            logger: Logger::new(path),
        }
    }

    /// Built by the runner, once per test, with `path` the test's
    /// registered name (D49 — UVM's fixed `uvm_test_top` is not ported).
    pub fn new(path: &str, dut: HierarchyHandle, seed: u64) -> RustdvCtx {
        RustdvCtx {
            dut,
            seed,
            objections: ObjectionRegistry::new(),
            logger: Logger::new(path),
        }
    }

    /// A child context: same services, path extended by `name` (D9). The
    /// phase traversals hand each child its own context so a component's
    /// `ctx.info()` logs the path the walk derived, never a stored string.
    pub fn child(&self, name: &str) -> RustdvCtx {
        RustdvCtx {
            dut: self.dut,
            seed: self.seed,
            objections: self.objections.clone(),
            // Segment extension, not string concatenation: the child's path is
            // the parent's plus one name, so it cannot be malformed (D7).
            logger: Logger::at(self.logger.rustdv_path().child(name)),
        }
    }

    pub fn dut(&self) -> HierarchyHandle {
        self.dut
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// A deterministic RNG seeded from RUSTDV_RANDOM_SEED + test index.
    pub fn rng(&self) -> Rng {
        Rng::new(self.seed)
    }

    /// This component's path — derived by the walk, never stored on the
    /// component itself (D7).
    pub fn path(&self) -> &str {
        self.logger.path()
    }

    /// This component's path as segments. The connection registry (D83)
    /// addresses components by this, so a port key cannot be hand-typed.
    pub fn rustdv_path(&self) -> &rustdv_sim::RustdvPath {
        self.logger.rustdv_path()
    }

    // --- Path-aware logging (D7's first real appearance) ------------------
    //
    // `log::info(...)` reaches a global sink with no idea who called it, and
    // a hand-typed `Logger::new("env.loga")` silently lies the moment a
    // component moves. These do not, because the path came from the walk.

    pub fn debug(&self, msg: &str) {
        self.logger.debug(msg);
    }
    pub fn info(&self, msg: &str) {
        self.logger.info(msg);
    }
    pub fn warning(&self, msg: &str) {
        self.logger.warning(msg);
    }
    pub fn error(&self, msg: &str) {
        self.logger.error(msg);
    }
    pub fn critical(&self, msg: &str) {
        self.logger.critical(msg);
    }

    /// The logger itself, for code that wants to hold one.
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    // --- Hierarchical logging control (pyuvm's *_hier methods) ------------
    //
    // Each applies to this component and everything below it. Note what is
    // *missing* from every signature: a path. pyuvm's `set_logging_level_hier`
    // is a method on the component and knows its own name; ours knows it
    // because the walk handed the context its path (D7). The alternative —
    // `set_level_for("uvm_test_top.comp", ..)` typed by hand — is a string
    // nobody checks, that silently addresses the wrong subtree the moment a
    // component is renamed or moved.

    /// Port of `set_logging_level_hier(level)`.
    pub fn set_logging_level_hier(&self, level: Level) {
        rustdv_sim::log::set_level_for(self.path(), level);
    }

    /// Port of `disable_logging_hier()`.
    pub fn disable_logging_hier(&self) {
        rustdv_sim::log::set_level_for(self.path(), Level::Off);
    }

    /// Port of `add_logging_handler_hier(logging.FileHandler(path))` —
    /// this subtree's messages are also written to `file`.
    pub fn add_file_handler_hier(&self, file: &str, append: bool) -> std::io::Result<()> {
        rustdv_sim::log::add_file_for(self.path(), file, append)
    }

    /// Port of `remove_streaming_handler_hier()` — stop printing this
    /// subtree to the console (file handlers keep receiving it).
    pub fn remove_console_hier(&self) {
        rustdv_sim::log::set_console_for(self.path(), false);
    }

    // --- Objections -------------------------------------------------------

    /// Port of raise_objection, returning a guard whose Drop is
    /// drop_objection (pyuvm: uvm_component.objection()).
    pub fn raise_objection(&self, description: &str) -> ObjectionGuard {
        self.objections.raise(description)
    }

    pub fn objections(&self) -> &ObjectionRegistry {
        &self.objections
    }

    /// Wait until every raised objection has been dropped. Logs the pyuvm
    /// "you never objected" warning if nothing was ever raised.
    pub async fn all_objections_dropped(&self) {
        self.objections.wait_all_dropped().await;
    }
}

// ===========================================================================
// The lifecycle
// ===========================================================================

/// The UVM phase lifecycle (design-doc §5.3, D51), restored in full. Nine
/// phases, each a method with a default no-op body — override only what you
/// use, exactly as pyuvm's `uvm_component` does. **`build` and `connect`
/// are real phases again**, not the "constructor conventions" R3 collapsed
/// them into; restoring them is the point of this chapter.
///
/// Every phase receives the context so it can log with the component's
/// derived path (D52/D7). The runner drives the whole sequence over the
/// tree (see [`run_component_test`]), so a component author never calls a
/// phase by hand.
pub trait Component {
    /// 1. `build` — top-down. Where a component constructs its children
    ///    (D6); the gap between "a component exists" and "its children exist"
    ///    that all late binding lives in.
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 2. `connect` — bottom-up. Wire children together once they exist.
    fn connect(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 3. `end_of_elaboration` — top-down. The hierarchy is final.
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 4. `start_of_simulation` — top-down. Last chance before time moves.
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 5. `run` — bottom-up, async, objection-gated. The test body; `Err`
    ///    fails the test.
    ///
    /// `async fn` in a trait costs dyn-compatibility, which is why the sync
    /// phases are mirrored onto [`DynPhases`] for traversal (D48).
    #[allow(async_fn_in_trait)]
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _ = ctx;
        Ok(())
    }
    /// 6. `extract` — top-down, post-run.
    fn extract(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 7. `check` — top-down. Report failures into the sink.
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let _ = (ctx, errors);
    }
    /// 8. `report` — top-down.
    fn report(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 9. `final_phase` — top-down. (`final` is a Rust keyword.)
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }

    /// **Transitional spawn hook**, pre-dating the restored `run`
    /// traversal. `tinyalu_tb` and the not-yet-converted chapters still
    /// spawn free-running behavior here; it folds into `run` as each
    /// converts. Not part of the nine-phase lifecycle.
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }

    // --- Factory (D75) ----------------------------------------------------

    /// This type's registered short name, used as the factory override key.
    /// The last path segment of the type name (`Foo` from
    /// `crate::mod::Foo`), which matches the name `#[derive(Component)]`
    /// registers.
    fn comp_name() -> &'static str
    where
        Self: Sized,
    {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }

    /// Build this component the normal way and drop it in an [`crate::factory::RustdvComp`]
    /// slot — the analogue of UVM's `new` (D75). Not overridable.
    fn new_comp() -> crate::factory::RustdvComp
    where
        Self: Sized + Default + ComponentNode + 'static,
    {
        crate::factory::RustdvComp::fixed(Box::new(Self::default()))
    }

    /// Build this component through the factory — the analogue of UVM's
    /// `create` (D75). The default is built now and the slot is flagged; the
    /// build walk swaps in an override if one applies.
    fn create_comp() -> crate::factory::RustdvComp
    where
        Self: Sized + Default + ComponentNode + 'static,
    {
        crate::factory::RustdvComp::overridable(Box::new(Self::default()), Self::comp_name())
    }
}

/// Dyn-safe mirror of [`Component`]'s non-async phases (D48).
///
/// `Component` stopped being dyn-compatible the moment `run` became an
/// `async fn`, and `ComponentNode` needs a dyn-safe supertrait to walk a
/// tree of `&mut dyn` children. The blanket impl means users never write
/// this: they override the phases on `Component`, and the distinct method
/// names keep `component.extract(..)` unambiguous.
pub trait DynPhases {
    fn dyn_build(&mut self, ctx: &mut RustdvCtx);
    fn dyn_connect(&mut self, ctx: &mut RustdvCtx);
    fn dyn_end_of_elaboration(&mut self, ctx: &mut RustdvCtx);
    fn dyn_start_of_simulation(&mut self, ctx: &mut RustdvCtx);
    /// The async `run`, boxed so it can be awaited behind `dyn` (D48). This
    /// is the object-safe shim `run_all` needs to fire each component's run.
    fn dyn_run<'a>(
        &'a mut self,
        ctx: &'a mut RustdvCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>>;
    fn dyn_extract(&mut self, ctx: &mut RustdvCtx);
    fn dyn_check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink);
    fn dyn_report(&mut self, ctx: &mut RustdvCtx);
    fn dyn_final(&mut self, ctx: &mut RustdvCtx);
    fn dyn_start(&mut self, ctx: &mut RustdvCtx);
}

impl<T: Component> DynPhases for T {
    fn dyn_build(&mut self, ctx: &mut RustdvCtx) {
        Component::build(self, ctx)
    }
    fn dyn_run<'a>(
        &'a mut self,
        ctx: &'a mut RustdvCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>> {
        Box::pin(Component::run(self, ctx))
    }
    fn dyn_connect(&mut self, ctx: &mut RustdvCtx) {
        Component::connect(self, ctx)
    }
    fn dyn_end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        Component::end_of_elaboration(self, ctx)
    }
    fn dyn_start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        Component::start_of_simulation(self, ctx)
    }
    fn dyn_extract(&mut self, ctx: &mut RustdvCtx) {
        Component::extract(self, ctx)
    }
    fn dyn_check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        Component::check(self, ctx, errors)
    }
    fn dyn_report(&mut self, ctx: &mut RustdvCtx) {
        Component::report(self, ctx)
    }
    fn dyn_final(&mut self, ctx: &mut RustdvCtx) {
        Component::final_phase(self, ctx)
    }
    fn dyn_start(&mut self, ctx: &mut RustdvCtx) {
        Component::start(self, ctx)
    }
}

/// Structural traversal over the ownership tree (design-doc D5.2).
/// Generated by `#[derive(Component)]` for structs whose children are
/// fields marked `#[component]`; hand-implementable by design
/// (OQ-15: the derive is convenience, not requirement).
pub trait ComponentNode: DynPhases {
    /// The component's type-level name (hierarchical path is synthesized
    /// from field names during traversal).
    fn node_name(&self) -> &'static str;

    /// Direct children as (field-derived name, node) pairs, in declaration
    /// order. An **owned Vec**, not a `visit_children`-style sync callback:
    /// [`run_all`] awaits inside the walk, and a higher-ranked closure
    /// cannot yield a child borrow that outlives the call, so the borrows
    /// have to come back in a value the caller holds. An `Option<T>` child
    /// created during `build` (D6) appears here only once it is `Some`.
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))>;

    /// This node's port binding cell of the given name, erased (D83).
    ///
    /// **This method is the cast Rust does not have.** A parent holds its
    /// children as `dyn ComponentNode` and cannot recover their concrete
    /// types, but it does not need to: it needs one port, by name, and a trait
    /// method reaches through erasure by definition. `#[derive(Component)]`
    /// generates the match arm for each `#[port(..)]` field; the default is
    /// `None`, for components that declare no ports.
    fn port_slot(&self, name: &str) -> Option<std::rc::Rc<dyn std::any::Any>> {
        let _ = name;
        None
    }

    /// Every port this node declares, for the elaboration report.
    fn port_infos(&self) -> Vec<crate::port::PortInfo> {
        Vec::new()
    }

    /// Resolve factory overrides for this node's `RustdvComp` fields (D75).
    /// The derive generates this to call `field.resolve(ctx, "field")` for
    /// each `RustdvComp` field; the default is a no-op for nodes with none.
    /// Called by [`build_all`] after `build`, before descending — so a
    /// swapped-out default's own phases never run.
    fn resolve_children(&mut self, ctx: &RustdvCtx) {
        let _ = ctx;
    }

    /// Move this node's `RustdvComp` children **out**, for the run phase (D82b).
    ///
    /// Returns owned boxes with no borrow relationship to `self`, which is what
    /// lets [`run_all`] drive a parent's own `run` concurrently with its
    /// children's. The derive generates this for `RustdvComp` fields; other
    /// child shapes (`Option<T>`, `Vec<T>`, plain `T`) stay in place and are
    /// reached through [`ComponentNode::children_mut`] as before.
    ///
    /// The default returns nothing, so a hand-written `ComponentNode` keeps the
    /// old behaviour and still compiles.
    fn take_children(&mut self) -> Vec<(String, Box<dyn ComponentNode>)> {
        Vec::new()
    }

    /// Put back what [`ComponentNode::take_children`] removed, in the same order.
    /// Called unconditionally after the run phase — including on error — so the
    /// post-run phases walk a whole tree.
    fn restore_children(&mut self, taken: Vec<(String, Box<dyn ComponentNode>)>) {
        let _ = taken;
    }
}

// ---------------------------------------------------------------------------
// Phase traversals (pyuvm order, D34): build top-down, connect bottom-up,
// run bottom-up, the elaboration and post-run phases top-down.
//
// Each child is walked with its own context (path extended, D9), so a
// component always logs under the path the walk gave it. Top-down = the
// node acts, then its children; bottom-up = children first, then the node.
// ---------------------------------------------------------------------------

/// Top-down: `build` a node, then build the children it just created (D6).
/// Reading `children_mut` *after* `build` is what lets a parent construct
/// them in its own build phase and have the walk descend into them.
pub fn build_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_build(ctx);
    // Swap in factory overrides for this node's `RustdvComp` children now, while
    // the node is accessible as its concrete type, and *before* descending —
    // so a replaced default's own build never runs (D75).
    node.resolve_children(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        build_all(child, &mut cctx);
    }
}

/// Bottom-up: children `connect` before parents.
pub fn connect_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        connect_all(child, &mut cctx);
    }
    node.dyn_connect(ctx);
}

/// Every required port in the tree that nobody connected, as
/// `path.name (kind)`.
///
/// The whole tree is swept and **all** the misses are reported at once (D85),
/// which is the point of declaring ports rather than reaching for handles:
/// pyuvm discovers a missing connection lazily, at first use, as an attribute
/// error deep inside a run phase. Analysis ports are exempt — a monitor that
/// nobody subscribes to is a legitimate testbench.
pub fn unconnected_ports(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) -> Vec<String> {
    let mut out = Vec::new();
    let path = ctx.path().to_string();
    for info in node.port_infos() {
        if info.required && !info.connected {
            let owner = if path.is_empty() {
                String::from("(top)")
            } else {
                path.clone()
            };
            out.push(format!("{owner}.{} ({})", info.name, info.kind));
        }
    }
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        out.extend(unconnected_ports(child, &mut cctx));
    }
    out
}

/// Run the connection sweep and turn any misses into one error listing them
/// all. Called by the runner between `connect` and `end_of_elaboration`.
pub fn check_connections(
    node: &mut dyn ComponentNode,
    ctx: &mut RustdvCtx,
) -> Result<(), TestError> {
    let missing = unconnected_ports(node, ctx);
    if missing.is_empty() {
        return Ok(());
    }
    let mut msg = String::from("these TLM ports were declared but never connected:");
    for m in &missing {
        msg.push_str("\n  ");
        msg.push_str(m);
    }
    // A classified failure, so a test can assert it failed for *this* reason:
    // `#[rustdv::test(expect_error = "tlm_unconnected_port")]`.
    Err(TestError::with_kind(msg, "tlm_unconnected_port"))
}

/// Top-down.
pub fn end_of_elaboration_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_end_of_elaboration(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        end_of_elaboration_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn start_of_simulation_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_start_of_simulation(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        start_of_simulation_all(child, &mut cctx);
    }
}

/// One component's own `run`, ended by the objection consensus.
///
/// **Every run body races the drained event here, at the leaf, rather than the
/// whole tree racing it at the top.** The difference matters because losing a
/// race means being *dropped*: a top-level race drops the entire `run_all`
/// future, and with it the children [`take_children`] moved out — so
/// extract/check/report would walk a tree whose components had been destroyed
/// mid-phase, silently, and a scoreboard's `check` would never run. Racing per
/// component instead lets every level of the walk return normally and put its
/// children back.
///
/// A run body that loops forever (a driver, a monitor) is dropped when the
/// consensus is reached, which is what UVM does to its forked `run_phase`
/// processes.
async fn run_one<'a>(
    node: &'a mut dyn ComponentNode,
    ctx: &'a mut RustdvCtx,
) -> Result<(), TestError> {
    let objections = ctx.objections().clone();
    match rustdv_sim::first2(node.dyn_run(ctx), objections.wait_drained_event()).await {
        rustdv_sim::Either::First(r) => r,
        // The consensus ended the phase: this component's run did not fail,
        // it was stopped.
        rustdv_sim::Either::Second(()) => Ok(()),
    }
}

/// Bottom-up: every component's `run` fires, children first (D48). The walk
/// is boxed-recursive because `dyn_run` yields a boxed future we await, and
/// the children's borrows are held across those awaits.
///
/// **Concurrent (D82).** Every component's `run` makes progress together —
/// the analog of UVM forking each `run_phase`. The children's runs are joined
/// (SystemVerilog's `fork...join`), and the node's own `run` joins them, so a
/// producer that blocks on a full FIFO and a consumer that drains it can both
/// proceed. Sequential awaiting was the earlier behaviour and deadlocked on
/// exactly that shape.
///
/// The futures **borrow** the tree rather than being spawned: `spawn` is
/// `'static`-bound and would force the component tree into `Rc`/`RefCell`,
/// whereas this scope already owns it. `spawn` stays for work that must
/// outlive the phase (BFM loops, monitor collectors — D59/D61).
///
/// **Scope: siblings are concurrent; a node's own `run` follows its subtree.**
/// All of a node's children (and their subtrees) run joined together, then the
/// node's own `run` body executes. That is what Rust's borrow rules allow:
/// `Component::run` takes `&mut self`, which *includes* the child fields, so
/// one `&mut node` cannot be split into "this node's own state" and "its
/// children" — a parent's run future and its children's run futures cannot
/// coexist.
///
/// **D82b lifts that limit for `RustdvComp` children.** A `RustdvComp` slot
/// holds a `Box`, so the box can be *moved out* of the parent for the duration
/// of the run phase. Once out, it has no borrow relationship to the parent, and
/// the two futures can be driven together. The boxes go back before the
/// post-run phases walk the tree.
///
/// So a node's run proceeds in two steps:
///
/// 1. **In-place children first** — `Option<T>`, `Vec<T>` and plain `T` fields
///    cannot be moved out of their parent, so they keep the earlier behaviour:
///    joined with each other, completing before the parent's own run begins.
///    Legacy chapters (ch24, ch25, `tinyalu_tb`) are all of this shape and hold
///    parents with no run body, so nothing changes for them.
/// 2. **Taken children joined *with* the parent's own run** — the shape D78
///    prescribes for all new code, and the one the sequence chapters need.
pub fn run_all<'a>(
    node: &'a mut dyn ComponentNode,
    ctx: &'a mut RustdvCtx,
) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>> {
    Box::pin(async move {
        // Step 1: move the factory children out **first**, so this node's own
        // run can be concurrent with theirs (D82b) — and so the in-place walk
        // below does not see them and run them to completion instead.
        let mut taken = node.take_children();

        // Step 2: in-place children (legacy shapes that cannot be moved out).
        // The block scopes their borrow of `node` so it ends before anything
        // below reborrows. Each child's context clone carries its derived path
        // (D9) and is moved into the future that uses it, so no borrows overlap.
        {
            let children: Vec<RunFuture<'_>> = node
                .children_mut()
                .into_iter()
                .map(|(name, child)| {
                    let cctx = ctx.child(&name);
                    Box::pin(async move {
                        let mut cctx = cctx;
                        run_all(child, &mut cctx).await
                    }) as RunFuture<'_>
                })
                .collect();

            // First error wins; the others keep running until the join is done.
            if !children.is_empty() {
                for r in rustdv_sim::join_all(children).await {
                    if r.is_err() {
                        node.restore_children(taken);
                        return r;
                    }
                }
            }
        }

        // Step 3: the taken children join this node's own run.
        if taken.is_empty() {
            return run_one(node, ctx).await;
        }

        let outcome = {
            let mut futs: Vec<RunFuture<'_>> = Vec::new();
            for (name, child) in taken.iter_mut() {
                let cctx = ctx.child(name);
                futs.push(Box::pin(async move {
                    let mut cctx = cctx;
                    run_all(&mut **child, &mut cctx).await
                }));
            }
            // `taken` and `node` are now separate values, so the parent's own
            // run joins its children's instead of following them.
            futs.push(Box::pin(run_one(node, ctx)));

            let mut first_err = Ok(());
            for r in rustdv_sim::join_all(futs).await {
                if first_err.is_ok() {
                    first_err = r;
                }
            }
            first_err
        };

        // Unconditional — including on error — so extract/check/report walk a
        // whole tree.
        node.restore_children(taken);
        outcome
    })
}

/// Bottom-up: children start before parents (transitional spawn hook).
pub fn start_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        start_all(child, &mut cctx);
    }
    node.dyn_start(ctx);
}

/// Top-down.
pub fn extract_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_extract(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        extract_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn check_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx, sink: &mut CheckSink) {
    node.dyn_check(ctx, sink);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        check_all(child, &mut cctx, sink);
    }
}

/// Top-down.
pub fn report_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_report(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        report_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn final_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_final(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        final_all(child, &mut cctx);
    }
}

/// The standard post-run tail: extract → check → report → final, returning
/// `Err` if any check failed (an `Err` fails the test, design-doc §0.6).
pub fn run_extract_check_report(
    node: &mut dyn ComponentNode,
    ctx: &mut RustdvCtx,
) -> Result<(), String> {
    extract_all(node, ctx);
    let mut sink = CheckSink::new();
    check_all(node, ctx, &mut sink);
    report_all(node, ctx);
    final_all(node, ctx);
    sink.into_result()
}

/// The full phaser: drive every UVM phase over a component tree, in order
/// (D51) — the analog of pyuvm handing the test class to its phaser. The
/// runner calls this for a `#[rustdv::test]` struct, so the test body is
/// just the phase methods; no hand-rolled `start_all`.
pub async fn run_component_test<T: Component + ComponentNode>(
    test: &mut T,
    ctx: &mut RustdvCtx,
) -> Result<(), TestError> {
    // Writes made during build take depth-scaled precedence, so a parent
    // outranks a child even though build is top-down and the parent
    // therefore writes first (D13, tier 2).
    crate::config::set_in_build(true);
    build_all(test, ctx);
    crate::config::set_in_build(false);

    connect_all(test, ctx);

    // Elaboration check: every declared port must be wired before anything
    // runs (D22/D85). The whole tree is swept and every miss is named at once,
    // where pyuvm finds the first one lazily, at use, deep inside a run phase.
    check_connections(test, ctx)?;

    end_of_elaboration_all(test, ctx);
    start_of_simulation_all(test, ctx);

    // The run phase ends when the last objection drops (UVM), or when every
    // run body has returned — whichever comes first (D82). Racing the two is
    // what lets a responder loop (a driver or monitor that never returns) end
    // with the phase instead of hanging the test. Unfinished runs are dropped
    // silently, as UVM kills its forked run processes.
    //
    // A test that never objected is not made to wait for consensus (D46), so
    // in that case the run tree alone decides.
    // The run phase ends when the objection consensus is reached or when every
    // run body has returned, whichever comes first — but the race is run *per
    // component*, inside `run_all` (see `run_one`), not around the whole tree.
    // Racing the whole tree here would drop it mid-phase and destroy the
    // components before extract/check/report could walk them.
    let run_result = run_all(test, ctx).await;

    let post = run_extract_check_report(test, ctx).map_err(TestError::from);
    run_result.and(post)
}

/// Debug printer: the child walker serving pyuvm's hierarchy print
/// (design-doc §5.2).
pub fn print_hierarchy(node: &mut dyn ComponentNode) {
    fn rec(node: &mut dyn ComponentNode, path: &str) {
        rustdv_sim::log::info(&format!("{path} ({})", node.node_name()));
        let parent = path.to_string();
        for (name, child) in node.children_mut() {
            rec(child, &format!("{parent}.{name}"));
        }
    }
    rec(node, "top");
}

// ===========================================================================
// Tests — no simulator. The walk is ordinary tree traversal; only `run`
// awaits, and these run bodies return immediately.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::RustdvComp;
    use rustdv_sim::testing::block_on;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Trace = Rc<RefCell<Vec<String>>>;

    thread_local! {
        static TRACE: Trace = Rc::new(RefCell::new(Vec::new()));
    }

    fn note(s: String) {
        TRACE.with(|t| t.borrow_mut().push(s));
    }
    fn trace() -> Vec<String> {
        TRACE.with(|t| t.borrow().clone())
    }
    fn reset() {
        TRACE.with(|t| t.borrow_mut().clear());
    }

    /// A leaf that records every phase it is given, with its own path — so
    /// the test can assert both the order *and* that the path was derived by
    /// the walk (D7) rather than stored.
    #[derive(Default)]
    struct Leaf;

    impl Component for Leaf {
        fn build(&mut self, ctx: &mut RustdvCtx) {
            note(format!("build {}", ctx.path()));
        }
        fn connect(&mut self, ctx: &mut RustdvCtx) {
            note(format!("connect {}", ctx.path()));
        }
        fn check(&mut self, ctx: &mut RustdvCtx, _e: &mut CheckSink) {
            note(format!("check {}", ctx.path()));
        }
        async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
            note(format!("run {}", ctx.path()));
            Ok(())
        }
    }

    impl ComponentNode for Leaf {
        fn node_name(&self) -> &'static str {
            "Leaf"
        }
        fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
            Vec::new()
        }
    }

    /// A parent that creates its children in `build` — two-stage construction
    /// (D6). The walk must descend into what build just made.
    #[derive(Default)]
    struct Parent {
        first: Option<Leaf>,
        second: Option<Leaf>,
    }

    impl Component for Parent {
        fn build(&mut self, ctx: &mut RustdvCtx) {
            note(format!("build {}", ctx.path()));
            self.first = Some(Leaf);
            self.second = Some(Leaf);
        }
        fn connect(&mut self, ctx: &mut RustdvCtx) {
            note(format!("connect {}", ctx.path()));
        }
    }

    impl ComponentNode for Parent {
        fn node_name(&self) -> &'static str {
            "Parent"
        }
        fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
            let mut out: Vec<(String, &mut (dyn ComponentNode + 'static))> = Vec::new();
            if let Some(c) = self.first.as_mut() {
                out.push((String::from("first"), c));
            }
            if let Some(c) = self.second.as_mut() {
                out.push((String::from("second"), c));
            }
            out
        }
    }

    /// Build is **top-down**: a parent acts, then the children it just made.
    /// That gap is where every late-binding mechanism lives (D5).
    #[test]
    fn build_is_top_down_and_descends_into_what_it_created() {
        reset();
        let mut root = Parent::default();
        let mut ctx = RustdvCtx::for_test("top");
        build_all(&mut root, &mut ctx);
        assert_eq!(
            trace(),
            vec!["build top", "build top.first", "build top.second"],
            "parent first, then the children it created in its own build"
        );
    }

    /// Connect is **bottom-up**: children are wired before their parent.
    #[test]
    fn connect_is_bottom_up() {
        reset();
        let mut root = Parent::default();
        let mut ctx = RustdvCtx::for_test("top");
        build_all(&mut root, &mut ctx);
        reset();
        connect_all(&mut root, &mut ctx);
        assert_eq!(
            trace(),
            vec!["connect top.first", "connect top.second", "connect top"]
        );
    }

    /// D7: the path comes from the field name via the walk. Rename the field
    /// and the path follows — which a hand-typed `Logger::new("top.first")`
    /// would not.
    #[test]
    fn paths_are_derived_from_field_names() {
        reset();
        let mut root = Parent::default();
        let mut ctx = RustdvCtx::for_test("alu_test");
        build_all(&mut root, &mut ctx);
        assert!(trace().contains(&String::from("build alu_test.first")));
        assert!(trace().contains(&String::from("build alu_test.second")));
    }

    #[test]
    fn an_option_child_appears_only_once_some() {
        let mut root = Parent::default();
        assert!(
            root.children_mut().is_empty(),
            "declared but not yet built (D6)"
        );
        let mut ctx = RustdvCtx::for_test("top");
        build_all(&mut root, &mut ctx);
        assert_eq!(root.children_mut().len(), 2);
    }

    #[test]
    fn every_component_runs() {
        reset();
        block_on(async {
            let mut root = Parent::default();
            let mut ctx = RustdvCtx::for_test("top");
            build_all(&mut root, &mut ctx);
            reset();
            run_all(&mut root, &mut ctx).await.unwrap();
        });
        let t = trace();
        assert!(t.contains(&String::from("run top.first")));
        assert!(t.contains(&String::from("run top.second")));
    }

    #[test]
    fn check_visits_the_whole_tree() {
        reset();
        let mut root = Parent::default();
        let mut ctx = RustdvCtx::for_test("top");
        build_all(&mut root, &mut ctx);
        reset();
        let mut sink = CheckSink::new();
        check_all(&mut root, &mut ctx, &mut sink);
        assert_eq!(trace().len(), 2, "both leaves were checked");
        assert!(sink.is_ok());
    }

    // --- D82b: children move out for the run phase, and come back ---------

    #[derive(Default)]
    struct FactoryParent {
        child: RustdvComp,
    }

    impl Component for FactoryParent {}

    impl ComponentNode for FactoryParent {
        fn node_name(&self) -> &'static str {
            "FactoryParent"
        }
        fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
            let mut out: Vec<(String, &mut (dyn ComponentNode + 'static))> = Vec::new();
            if let Some(n) = self.child.as_node_mut() {
                out.push((String::from("child"), n));
            }
            out
        }
        fn take_children(&mut self) -> Vec<(String, Box<dyn ComponentNode>)> {
            let mut out = Vec::new();
            if let Some(n) = self.child.take_node() {
                out.push((String::from("child"), n));
            }
            out
        }
        fn restore_children(&mut self, taken: Vec<(String, Box<dyn ComponentNode>)>) {
            for (_, node) in taken {
                self.child.put_node(node);
            }
        }
    }

    #[test]
    fn take_children_empties_the_slot_and_restore_refills_it() {
        let mut p = FactoryParent {
            child: RustdvComp::fixed(Box::new(Leaf)),
        };
        let taken = p.take_children();
        assert_eq!(taken.len(), 1);
        assert!(
            p.children_mut().is_empty(),
            "the slot is empty during the run phase"
        );
        p.restore_children(taken);
        assert_eq!(p.children_mut().len(), 1, "and full again for check/report");
    }

    /// D82c: restoration is unconditional, so the post-run phases always walk
    /// a whole tree — including when a run returned an error. A tree missing
    /// its children is how a scoreboard silently never runs.
    #[test]
    fn children_are_restored_even_when_a_run_fails() {
        #[derive(Default)]
        struct Failing;
        impl Component for Failing {
            async fn run(&mut self, _c: &mut RustdvCtx) -> Result<(), TestError> {
                Err(TestError::from(String::from("deliberate")))
            }
        }
        impl ComponentNode for Failing {
            fn node_name(&self) -> &'static str {
                "Failing"
            }
            fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
                Vec::new()
            }
        }

        block_on(async {
            let mut p = FactoryParent {
                child: RustdvComp::fixed(Box::new(Failing)),
            };
            let mut ctx = RustdvCtx::for_test("top");
            let outcome = run_all(&mut p, &mut ctx).await;
            assert!(outcome.is_err(), "the child's run failed");
            assert_eq!(p.children_mut().len(), 1, "and its child came back anyway");
        });
    }

    #[test]
    fn a_component_with_no_children_walks_cleanly() {
        reset();
        let mut leaf = Leaf;
        let mut ctx = RustdvCtx::for_test("solo");
        build_all(&mut leaf, &mut ctx);
        connect_all(&mut leaf, &mut ctx);
        assert_eq!(trace(), vec!["build solo", "connect solo"]);
    }
}
