# Component access vs. the `RustdvComp` factory — research & recommendation

*Prepared 2026-07-23 (overnight research task). Question from Ray: TLM connection
exposed that a parent cannot reach data members or methods inside a factory
(`RustdvComp`) component. Is that "one mouse of many"? Survey the book, the web, and
UVM VIP for real end-user cases that rely on reaching into user-defined
components, and recommend whether we must refactor the factory.*

## Verdict up front

**Keep `RustdvComp`. Do not refactor the factory to typed/role-trait handles.**

Every real cross-component-access case falls into one of three buckets:

1. **Already path/registry-addressed** (TLM connect, the sequencer for sequences)
   — handled by the connection-registry design we sketched for ch31, which is
   just the ConfigDB pattern generalized.
2. **Not a component at all** (sequences, transactions, config objects, the
   register model) — created and held as ordinary values, never erased.
3. **The anti-pattern UVM itself tells you to avoid** (direct `a.b.c.member`
   hierarchical references) — which `RustdvComp` structurally forecloses, which is a
   *feature*, not a bug.

I found **no** cross-component access pattern that is (a) common, (b) unavoidable,
and (c) requires a typed compile-time handle. The one mechanism that ever *wants*
a typed instance handle — instance-specific `uvm_callbacks::add(handle, cb)` — has
a typewide form that needs no handle, and the instance form is registry-addressable.

So the fix is not a factory rewrite; it is to **promote the connection registry
into the one general mechanism** by which a component publishes the interface it
chooses to expose, and others reach it by path. That is a superset of Ray's TLM
port-registry idea, reuses the ConfigDB machinery, and matches UVM best practice.

---

## What I surveyed

- **The Python book** (`Python4RTLVerification-master`): all 24 chapters, every
  `testbench.py`, grepped and read for cross-component access (`self.x.y`,
  `.connect(...)`, `.start(...)`, ConfigDB use, result/method reach).
- **pyuvm source**: `_s12` (TLM), `_s13_uvm_component` (the component API surface),
  `_s14_15` (sequences).
- **The UVM Primer** (`uvmprimer-master`): the analysis-port and put/get examples.
- **Web**: UVM VIP configuration/control, `uvm_callbacks`, virtual sequencers /
  `p_sequencer`, and the best-practice literature on hierarchical references vs.
  the config DB (sources at the bottom).

## Findings — the catalogue of cross-component access

### 1. `connect_phase` TLM wiring — the dominant case, registry-addressable

Every component testbench in the book wires children in the parent's
`connect_phase`:

```python
self.driver.seq_item_port.connect(self.seqr.seq_item_export)
self.cmd_mon.ap.connect(self.scoreboard.cmd_export)
self.cmd_mon.ap.connect(self.coverage.analysis_export)
self.result_mon.ap.connect(self.scoreboard.result_export)
```

The parent reaches *both* a child's port (`cmd_mon.ap`) and another child's
export/subscriber (`scoreboard.cmd_export`, `coverage`). This is exactly the case
that motivated the registry design: ports/exports self-register by (path, name)
during build; `connect` is a registry lookup, never a concrete field reach. **This
is the whole of the book's cross-component access, and the registry covers it.**

### 2. The sequencer for sequences — *already* travels through the ConfigDB

This is the finding that reframes everything. Sequences need a sequencer to run
on (`seq.start(seqr)`), and the book does **not** reach `env.agent.seqr` by
handle. It publishes the sequencer to the ConfigDB and sequences fetch it:

```python
# env.build_phase:
self.seqr = uvm_sequencer("seqr", self)
ConfigDB().set(None, "*", "SEQR", self.seqr)
# sequence.body (even virtual sequences, TB 8.0):
seqr = ConfigDB().get(None, "", "SEQR")
await seq.start(seqr)
```

So the single most important stimulus handle in UVM is *already* registry-passed.
Our design didn't invent this; pyuvm did. `RustdvComp` is no obstacle here at all.

### 3. Sequences, transactions, config objects, RAL — not components

- **Sequences are `uvm_object`s, not components.** The user creates one locally
  (`OpSeq("seq", aa, bb, op)`), starts it on a sequencer, and reads its result
  (`seq.result`). No tree, no erasure. The "programming interface" pattern (TB
  8.0 `do_add(seqr, aa, bb) -> result`) is all local objects + a registry-fetched
  sequencer.
- **Transactions** are plain data (our R1: structs with derives).
- **VIP configuration objects** are `uvm_object`s passed via `config_db` /
  `uvm_resource_db`; the user calls methods on the retrieved *object*, not on a
  tree component. That is the ConfigDB story we already have (D65–D68), and it is
  the primary way VIP shares control (Synopsys AMBA VIP, SmartDV, the Synopsys
  config white paper).
- **The register model (RAL)** is a data model retrieved via config_db; register
  access (`reg.write()`) is method calls on that model, not on the component tree.
  (pyuvm implements neither RAL nor callbacks — out of scope now — but even in
  full UVM the reg model is published, not hierarchically reached.)

None of these touch the `RustdvComp` boundary.

### 4. `uvm_callbacks` — the one mechanism that *can* want an instance handle

Callbacks let a user inject behavior into a pre-built component (error injection,
transaction mangling) without editing it. Registration:

- **Typewide**: `uvm_callbacks#(driver)::add(null, cb)` — applies to *all*
  instances of a type, **needs no handle**.
- **Instance**: `uvm_callbacks#(driver)::add(env.agent.driver, cb)` — needs a
  handle to that instance.

The component *invokes* callbacks internally (`\`uvm_do_callbacks`); the user only
supplies the callback object. So even the instance form is a *register-by-address*
operation — a natural fit for the same path registry (register a callback hook
against a component path). And the common form (typewide) needs nothing. This is
control-sharing *beyond* config_db, and it is still late-binding, not member reach.

### 5. Virtual sequencers / `p_sequencer` — references are *assigned*, not reached

A virtual sequencer holds references to the real sub-sequencers, and — per the
literature — "these references are assigned from top environment to the
non-virtual sequencers." That is dependency injection (late binding), the registry
pattern again. `p_sequencer` is "technically never required" — a typed convenience
handle to the parent sequencer, set up by a macro. Reaching sub-sequencers is the
one *sanctioned* direct reference, and it is exactly `seq.start(sub_seqr)` with the
sub-seqr obtained by assignment/registry.

### 6. Direct hierarchical references — explicitly discouraged by UVM itself

This is the decisive framing. The expert guidance is unambiguous:

> "In general, avoid having direct object references between components. However,
> there are some places where a direct reference to another component is
> acceptable, such as where a virtual sequence makes a direct reference to a
> sequencer within an agent in order to start a child sequence." — Doulos,
> *Easier UVM Coding Guidelines*

> "Rather than using direct dot notation to access nested objects (e.g.,
> `smthgA.smthgB.smthgC.variableXYZ`), with uvm_config_db … different parts of the
> testbench can connect with an object, even if they don't know where it is in the
> hierarchy … makes components more reusable and portable." — config_db tutorials

So the very capability we "lost" — reaching `a.b.c.member` — is the capability UVM
best practice tells engineers **not** to use, because it creates tight coupling and
kills reuse. `RustdvComp` makes the anti-pattern un-writable and pushes users onto the
config_db/registry road they were supposed to take anyway. UVM's own hierarchy API
(`get_child`, `lookup`, `find`) is itself name/string-based and returns the *base*
`uvm_component`, requiring a `$cast` — i.e., UVM's built-in "reach a component"
path is already stringly-typed + runtime-checked, exactly like our registry.

## Assessment — is `RustdvComp` viable?

Yes. Mapping every case onto the design:

| Access pattern | Needs typed member reach? | How rustdv serves it |
|---|---|---|
| TLM port/export connect | no | connection registry (ch31 design) |
| Sequencer for `seq.start` | no | ConfigDB (as pyuvm does) |
| Sequence results / API | no | sequences are local objects |
| Config objects / knobs | no | ConfigDB (D65–D68) |
| Register model (RAL) | no | published model via ConfigDB |
| Callbacks (typewide) | no | type registry (D73) |
| Callbacks (instance) | address, not reach | path registry |
| Virtual seqr sub-seqrs | no (injected) | ConfigDB / registry |
| `a.b.c.member` direct ref | yes | **anti-pattern — intentionally unavailable** |

The only column that needs a typed member reach is the one UVM tells you not to
write. Everything a good testbench actually does is late-binding, and late binding
is precisely what the ConfigDB/registry provides and what D3 says must stay dynamic.

## Recommendation

1. **Do not refactor the factory.** `RustdvComp` + universal registration (D69–D75)
   stands. The over-erasure is only a problem if you need typed cross-component
   member access, and the evidence says good testbenches don't (and shouldn't).

2. **Promote the connection registry into the general "published interface"
   mechanism**, built on the ConfigDB. A component publishes, by (path, name), the
   handles it chooses to expose — TLM ports/exports (Ray's `#[port(...)]`), its
   sequencer, later a callback hook or a control port. Everyone else reaches those
   by path. One mechanism, already per-test-cleared, already the thing pyuvm uses
   for the sequencer. Ray's TLM design is the first and defining instance of it.

3. **State the costs honestly** (for the book, D3/D4):
   - Cross-component interfaces are reached by (path, name) with a **runtime** type
     check at connect/lookup — not compile time. This matches D22 ("late binding
     and static completeness are mutually exclusive") and pyuvm's own duck-typed
     check. Make the errors loud and specific (name the path and expected type).
   - A path typo is a runtime/elaboration error, not a compile error. Mitigate with
     macro-generated name constants (`CompStruct::PUT_PORT`) so the common cases
     are typo-proof.
   - `RustdvComp` genuinely cannot do `env.agent.driver.method()`. We frame that as
     the intended enforcement of loose coupling, and point users to the config
     DB / control ports — exactly UVM's own advice.

4. **Keep one escape hatch in the back pocket, unused until proven necessary.** If
   a real case ever needs a *typed* interface on a specific component, a component
   can publish a **role-trait handle** — `Rc<RefCell<dyn SomeRole>>` — into the
   registry under a path. That gives typed, dynamically-dispatched access to that
   one interface *without* abandoning `RustdvComp` or forking the factory. It is the
   SV "base-class handle" idea applied surgically, on demand, not wholesale.

## What would change this recommendation (falsifiable)

I'd reopen the factory question if we hit a use case that is **common**,
**unavoidable**, and needs **typed compile-time** access to another component's
members that cannot be reasonably expressed as a published-by-path interface. I did
not find one in the book, the Primer, pyuvm, or the VIP/best-practice literature.
The closest — instance callbacks and `p_sequencer` — are both address-based and
both have handle-free or injected forms. If the register model (RAL) or a
scoreboard-to-refmodel coupling later argues otherwise, the role-trait escape hatch
(rec. 4) absorbs it without a rewrite.

## Concrete next step for ch31 (TB 6.0)

Write the aspirational example Ray sketched, and make it compile/run on Icarus
before touching the real TinyALU:

```rust
// components declare ports; the macro registers each by (ctx path, field name)
struct Producer { #[port(put)] myput: PutPort<Cmd> }
struct Consumer { #[port(get)] myget: GetPort<Cmd> }

// FIFO is reachable (concrete or published); its exports resolve ports by path
fifo.put_export().connect((comp1, "myput"));
fifo.get_export().connect((comp2, "myget"));
```

Then build the real monitors → scoreboard/coverage analysis wiring on the same
registry, and only after that retrofit `tinyalu_tb`. The registry is the general
mechanism; TLM is its first customer.

---

## Sources

- [Doulos — Detailed Explanation of the Easier UVM Coding Guidelines](https://www.doulos.com/knowhow/systemverilog/uvm/easier-uvm/easier-uvm-coding-guidelines/detailed-explanation-of-the-easier-uvm-coding-guidelines/) (avoid direct object references between components; sequencer reference is the sanctioned exception)
- [Using UVM Virtual Sequencers & Virtual Sequences — Clifford E. Cummings (DVCon)](https://dvcon-proceedings.org/wp-content/uploads/using-uvm-virtual-sequencers-virtual-sequences.pdf)
- [ChipVerify — UVM Virtual Sequencer](https://chipverify.com/uvm/uvm-virtual-sequencer)
- [VLSI Worlds — m_sequencer and p_sequencer in UVM](https://vlsiworlds.com/uvm/m_sequencer-and-p_sequencer-in-uvm/)
- [Verification Guide — UVM Callback](https://verificationguide.com/uvm/uvm-callback/) and [UVM Callback example](https://verificationguide.com/uvm/uvm-callback-example/)
- [Verification Academy — UVM 1.2 Register Callbacks (uvm_reg_cbs)](https://verificationacademy.com/verification-methodology-reference/uvm/docs_1.2/html/files/reg/uvm_reg_cbs-svh.html)
- [Synopsys — Mastering UVM for Effective AXI VIP Usage](https://www.synopsys.com/blogs/chip-design/mastering-uvm-axi-vip-usage.html)
- [Synopsys white paper — Hierarchical Testbench Configuration Using uvm_config_db](https://www.synopsys.com/content/dam/synopsys/services/whitepapers/hierarchical-testbench-configuration-using-uvm.pdf)
- [SmartDV — UVM Testbench Architecture & VIP Integration](https://www.smartdvtech.com/uvm-testbenches-verification-ip-integration/)
- [Medium (S. Katiyar) — ConfigDB: The Heart of UVM](https://medium.com/@shivamkatiyar274/configdb-the-heart-of-uvm-976086ec765a)
- Local: `pyuvm` `_s12_uvm_tlm_interfaces.py`, `_s13_uvm_component.py`, `_s14_15_python_sequences.py`; `Python4RTLVerification-master` chapters 35–43; `uvmprimer-master` 16/18.
