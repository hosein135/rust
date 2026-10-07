# Appendix D: What rustdv Provides

Every Part II listing opens with `use rustdv::prelude::*` — the analog of `import uvm_pkg::*` and `from pyuvm import *`. This appendix is the complete reference for what that line brings into scope, plus the macros. The Chapter column points to where each name is taught; the Toolkit page (before Chapter 15) groups the same names by job. A dash means the name is provided for completeness but this book's examples never need it.

## The prelude, alphabetically

| Name | What it is | Chapter |
|---|---|---|
| `Active` | the active/passive agent knob, read from the ConfigDb (pyuvm's `is_active` int, as an enum) | 40 |
| `AnalysisBus` | the broadcast hub; it stores nothing | 32 |
| `build_all` | drive `build` across a component tree (the runner's job) | 24 |
| `channel` | make a `(Sender, Receiver)` queue pair with a capacity | — |
| `check_all` | drive `check` across a tree | 24 |
| `CheckSink` | the collector `check` phases write failures into | 24 |
| `Clock` | a software clock driver — taught once, then retired in favor of BFMs that wait on edges | 17 |
| `Component` | the lifecycle trait: `build`, `connect`, `run`, and the other phase methods | 24 |
| `ComponentNode` | the tree-traversal trait `#[derive(Component)]` implements | 21, 24 |
| `ConfigDb` | path-addressed runtime configuration; `get` returns a `Result` naming the cause | 25, 27 |
| `connect_all` | drive `connect` across a tree | 24 |
| `create_seq` | build a sequence through the sequence factory | 36 |
| `Either` | the answer from racing two differently-typed futures | — |
| `end_of_elaboration_all` | phase driver | 24 |
| `Event` | set once; everyone waiting wakes (SV: named `event`) | 16 |
| `extract_all` | phase driver | 24 |
| `Factory` | the component registry: build by type or name, override by type, name, or instance | 29 |
| `final_all` | phase driver | 24 |
| `first2`, `first!` | race futures; the first to finish wins (SV: `fork...join_any`) | 16 |
| `GetPort` | the consuming end of a TLM connection | 31 |
| `HandleError` | what signal access returns instead of a crash | 17, 19 |
| `HierarchyHandle` | a handle to a scope in the design hierarchy | — |
| `join2`, `join!` | run futures together; wait for all (SV: `fork...join`) | 16 |
| `Lock` | mutual exclusion with an RAII guard (SV: a one-key `semaphore`) | 16 |
| `log` | the logging facade; policy is per hierarchy | 15, 26 |
| `Logic`, `LogicArray` | four-state values, scalar and vector | 17 |
| `LogicHandle` | a named DUT signal: read it, drive it; a typo'd name is an `Err` | 17 |
| `next_time_step` | trigger for the simulator's next time step | — |
| `NullTrigger` | the trigger that is ready the next time it is polled | 15 |
| `ObjectionGuard` | returned by `ctx.raise_objection`; the run phase ends when the last guard drops | 23 |
| `PeekPort` | the look-without-taking end of a TLM connection | 31 |
| `PortName`, `PortOwner` | how the elaboration check names an unconnected port | 31 |
| `print_hierarchy` | dump a component tree | — |
| `PublishPort` | the publishing end a component declares | 32 |
| `PutPort` | the producing end of a TLM connection | 31 |
| `Queue` | the sim-aware mailbox: bounded puts and empty gets block in simulated time (SV: `mailbox#(T)`) | 16 |
| `read_only`, `read_write` | scheduler-region triggers (cocotb's `ReadOnly`/`ReadWrite`) | — |
| `Receiver` | the getting end `channel` returns | — |
| `report_all` | phase driver | 24 |
| `Rng` | the deterministic per-test random source behind `ctx.rng()` | 20 |
| `run_component_test` | run a component tree as a self-contained test | — |
| `run_extract_check_report` | drive the closing phases together | — |
| `RustdvComp` | a slot holding any factory-built component | 29 |
| `RustdvCtx` | the context: path, logging, rng, DUT handle, objection — the framework, handed as an argument | 15 |
| `RustdvSeq` | a slot holding any factory-built sequence — `RustdvComp`'s parallel | 36 |
| `RustdvShared` | a cloneable handle to one shared object — `Rc<RefCell>` wearing the framework's name | 32 |
| `Sender` | the putting end `channel` returns | — |
| `SeqCtx` | the context a sequence `body` receives | 36 |
| `SeqError` | what a sequence can fail with | 36 |
| `SeqItem` | the bounds a sequence-item type must meet | 36 |
| `SeqItemExport`, `SeqItemPort` | the driver's side of the sequencer handshake | 36 |
| `Sequence` | the trait with one method, `body` — a test program, not a component | 36 |
| `Sequencer` | grants sequences their turns; feeds the driver | 36 |
| `set_seq_override` | change which sequence `create_seq` builds | 36 |
| `sim_time_ns` | the current simulated time | — |
| `SimDuration` | an amount of simulated time | 17 |
| `spawn`, `spawn_named` | launch a concurrent task; the named form stamps its log lines | 16 |
| `start_all` | drive the run phase across a tree | 24 |
| `start_of_simulation_all` | phase driver | 24 |
| `SubscribePort` | the subscribing end a component declares | 32 |
| `Subscriber` | the trait a subscriber implements once per stream | 32 |
| `TaskHandle` | what `spawn` returns: await it for the result, or `cancel()` it | 16 |
| `TestError` | the error a failing test returns; `Ok(())` is a pass | 15 |
| `Timer` | the simulated-time trigger: `Timer::ns(2).await` | 15 |
| `TlmFifo` | the FIFO two components share without learning each other's names | 31 |
| `TxnId` | the ticket `finish_item` returns; claims a response | 37, 38 |
| `with_timeout` | wrap an await with a deadline | — |

## The macros

| Name | What it does | Chapter |
|---|---|---|
| `#[rustdv::test]` | registers a test with the runner (cocotb: `@cocotb.test()`) | 15, 21 |
| `#[derive(Component)]` | writes the component-tree plumbing (SV: the `uvm_component_utils` family) | 21, 24 |
| `vpi_bootstrap!()` | one line per testbench crate: exports the entry points the simulator loads | 17 |
| `first!`, `join!` | variadic race and join | 16 |

## On the surface, outside the prelude

A few names live on the crate but not in the prelude; reach them as `rustdv::Name`.

| Name | What it is | Chapter |
|---|---|---|
| `ConfigError` | why a ConfigDb `get` failed, as a value | 28 |
| `ConnectError` | why a connection could not be made | 31 |
| `TlmFull`, `TlmEmpty`, `TlmError` | the channel layer's refusals: what `Sender::try_send` and `Receiver::try_recv` answer | — |
| `Maker` | the closure type the factory stores per registration | 29 |
| `ResponseQueue` | the store behind `get_response` — responses held for claiming, in order or by ticket (pyuvm's `ResponseQueue`) | — |
| `TimeoutError`, `TaskError`, `ValueError`, `AnyHandle`, `Executor`, `TestRegistration`, `top_module` | infrastructure corners a testbench rarely touches | — |

The whole of each layer is also re-exported for power users: `rustdv::sim`, `rustdv::runner`, `rustdv::gpi`.
