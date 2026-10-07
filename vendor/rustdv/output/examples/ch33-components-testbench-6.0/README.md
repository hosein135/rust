# Chapter 33: Components in Testbench 6.0 — figure map

**This chapter has no crate of its own.** It is a chapter of component
*definitions* — the 6.0 principle, one job per component — with no environment
and nothing to run. Chapter 34 wires these same components into a working
testbench, so they live in Chapter 34's crate and carry `// Chapter 33,
Figure N:` captions there (D91).

That is deliberate. D45 requires each chapter example to be self-contained:
factoring the components into a crate that Chapter 34 imports is the
cross-chapter import D45 dissolved. The Python book splits them
(`37_components_in_testbench_6.0/component_testbench.py` defines the classes,
`38_connections_in_testbench_6.0/testbench.py` imports them) and its
components chapter has no runnable test either.

| Figure | Title | Where |
|---|---|---|
| 1 | The Tester puts commands into a FIFO | `../ch34-connections-testbench-6.0/src/ch34_connections_testbench_6_0.rs` (`Tester`) |
| 2 | The Driver gets commands and drives the BFM | same file (`Driver`) |
| 3 | The command monitor watches the bus and broadcasts | same file (`CmdMonitor`) |
| 4 | The result monitor broadcasts results | same file (`ResultMonitor`) |
| 5 | The Scoreboard subscribes to BOTH streams | same file (`Scoreboard`) |
| 6 | Coverage subscribes to the command stream only | same file (`Coverage`) |

No tests, and no transcript — nothing here runs on its own. The transcript for
these components in action is in
[`../ch34-connections-testbench-6.0/README.md`](../ch34-connections-testbench-6.0/README.md).

## What this chapter proves

- **One job per component.** Each of the six either creates data and writes it
  to a port, or gets data from a port and processes it. Nothing does both, and
  nothing reaches for another component.
- **The stimulus and the observation sides are symmetric.** The Tester puts and
  the Driver gets, point-to-point through a `TlmFifo`; the monitors publish and
  the Scoreboard and Coverage subscribe, one-to-many through an `AnalysisBus`.
  Two shapes of TLM, one connection idiom (Chapters 31 and 32).
- **A component declares what it needs and is handed it.** Every one of the six
  declares `#[port(..)]` fields and nothing else — no handles to siblings, no
  knowledge of what is on the other end. Chapter 34 supplies the other ends.
