# Chapter 34: Connections in Testbench 6.0

Chapter 33 built six components that do not know each other — that was the point of building them that way, and it left the chapter with nothing it could run. This chapter introduces them to each other. One environment builds all six, wires every connection in one `connect` method, and runs the first fully-decoupled TinyALU testbench: version 6.0.

> **In the UVM...** we wired testbench 6.0 in `connect_phase()`: the tester's put port to one side of a `uvm_tlm_fifo`, the driver's get port to the other, and the monitors' analysis ports fanned out to the scoreboard and coverage — a diagram's worth of `connect()` calls.

The diagram is worth having before the calls:

<figure>
<svg viewBox="0 0 680 250" xmlns="http://www.w3.org/2000/svg" font-family="sans-serif" font-size="13">
  <defs>
    <marker id="carr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#888"/>
    </marker>
  </defs>
  <!-- stimulus lane -->
  <rect x="10" y="20" width="90" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="55" y="41" text-anchor="middle" fill="currentColor" font-weight="bold">Tester</text>
  <rect x="150" y="20" width="90" height="32" rx="4" fill="none" stroke="#888"/>
  <text x="195" y="41" text-anchor="middle" fill="currentColor">cmd_fifo</text>
  <rect x="290" y="20" width="90" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="335" y="41" text-anchor="middle" fill="currentColor" font-weight="bold">Driver</text>
  <rect x="430" y="20" width="70" height="32" rx="8" fill="none" stroke="#888"/>
  <text x="465" y="41" text-anchor="middle" fill="currentColor">BFM</text>
  <rect x="550" y="20" width="70" height="32" rx="8" fill="none" stroke="#888"/>
  <text x="585" y="41" text-anchor="middle" fill="currentColor">DUT</text>
  <line x1="100" y1="36" x2="147" y2="36" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="122" y="29" fill="currentColor" font-size="10">put</text>
  <line x1="240" y1="36" x2="287" y2="36" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="262" y="29" fill="currentColor" font-size="10">get</text>
  <line x1="380" y1="36" x2="427" y2="36" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <line x1="500" y1="36" x2="547" y2="36" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <!-- command broadcast lane -->
  <rect x="10" y="105" width="110" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="65" y="126" text-anchor="middle" fill="currentColor" font-weight="bold">CmdMonitor</text>
  <rect x="180" y="105" width="90" height="32" rx="4" fill="none" stroke="#888"/>
  <text x="225" y="126" text-anchor="middle" fill="currentColor">cmd_bus</text>
  <rect x="340" y="85" width="110" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="395" y="106" text-anchor="middle" fill="currentColor" font-weight="bold">Scoreboard</text>
  <rect x="340" y="130" width="110" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="395" y="151" text-anchor="middle" fill="currentColor" font-weight="bold">Coverage</text>
  <line x1="120" y1="121" x2="177" y2="121" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="140" y="114" fill="currentColor" font-size="10">pub</text>
  <line x1="270" y1="113" x2="337" y2="102" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="295" y="98" fill="currentColor" font-size="10">sub</text>
  <line x1="270" y1="129" x2="337" y2="145" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="295" y="148" fill="currentColor" font-size="10">sub</text>
  <!-- result broadcast lane -->
  <rect x="10" y="195" width="120" height="32" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="70" y="216" text-anchor="middle" fill="currentColor" font-weight="bold">ResultMonitor</text>
  <rect x="190" y="195" width="95" height="32" rx="4" fill="none" stroke="#888"/>
  <text x="237" y="216" text-anchor="middle" fill="currentColor">result_bus</text>
  <line x1="130" y1="211" x2="187" y2="211" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="150" y="204" fill="currentColor" font-size="10">pub</text>
  <line x1="285" y1="205" x2="337" y2="112" stroke="#888" stroke-width="1.5" marker-end="url(#carr)"/>
  <text x="320" y="175" fill="currentColor" font-size="10">sub</text>
</svg>
<figcaption><em>Figure 1: Testbench 6.0. One point-to-point lane for stimulus; two broadcast lanes for observation.</em></figcaption>
</figure>

Three lanes, two mechanisms. The stimulus lane is Chapter 31: Tester puts, Driver gets, and the `cmd_fifo` between them means neither knows the other exists. The observation lanes are Chapter 32: each monitor publishes into a bus that stores nothing, and the subscribers own what they keep. Coverage listens to the command bus alongside the scoreboard — and neither the scoreboard nor the monitor knows Coverage is there, which is the decoupling the hub buys.

## The environment

```rust
// Chapter 34, Figure 2: Build the components and the FIFOs; connect in one place
#[derive(Component, Default)]
struct AluEnv {
    #[component]
    tester: RustdvComp,
    #[component]
    driver: RustdvComp,
    #[component]
    cmd_mon: RustdvComp,
    #[component]
    result_mon: RustdvComp,
    #[component]
    scoreboard: RustdvComp,
    #[component]
    coverage: RustdvComp,
    #[component]
    cmd_fifo: TlmFifo<Command>,
    #[component]
    cmd_bus: AnalysisBus<CmdTuple>, // the command broadcast, two subscribers
    #[component]
    result_bus: AnalysisBus<u64>, // the result broadcast, one subscriber
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tester = Tester::new_comp();
        self.driver = Driver::new_comp();
        self.cmd_mon = CmdMonitor::new_comp();
        self.result_mon = ResultMonitor::new_comp();
        self.scoreboard = Scoreboard::new_comp();
        self.coverage = Coverage::new_comp();
        self.cmd_fifo = TlmFifo::new(1);
        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // stimulus: Tester --put--> cmd_fifo --get--> Driver
        self.cmd_fifo.put_export().connect(&self.tester, Tester::CMD_PORT);
        self.cmd_fifo.get_export().connect(&self.driver, Driver::CMD_PORT);

        // commands: CmdMonitor publishes; Scoreboard and Coverage subscribe
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);

        // results: ResultMonitor publishes; only the Scoreboard subscribes
        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

Read the `connect` method as a whole before reading any line of it: seven connections, and **every one is the same shape** — a concrete FIFO, a named export, `connect(component, PORT_NAME)`. Point-to-point traffic through a `TlmFifo`, broadcast through an `AnalysisBus`, and the idiom does not change between them; the only visible difference is that two subscribers connect to the same `sub_export()`. Nothing reaches into an erased child; every endpoint is a `RustdvComp`, and every connection resolves through the trait method that answers the same way for a child slot and for `self`. Chapter 33's six definitions plus these seven lines *are* testbench 6.0 — the figure-1 diagram, transcribed.

Two smaller notes. The `cmd_fifo` has depth 1, so the Tester cannot run ahead of the Driver — the same back-pressure lesson as Chapter 31's first transcript, now doing real work. And `start_of_simulation` starts the BFM's monitoring tasks after the whole tree is built and wired, in the phase whose position in the lifecycle exists for exactly this kind of "everything is ready, nothing has run" work.

## The test

```rust
// Chapter 34, Figure 3: The test is just the env
#[rustdv::test]
#[derive(Component, Default)]
struct AluTest {
    #[component]
    env: RustdvComp,
}

impl Component for AluTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }
}
```

The test constructs the BFM from the DUT handle, files it in the ConfigDb for every component that needs it, and builds the env. It has no `run` at all — for the first time in this book, the test contributes nothing to the run phase, because stimulus is the Tester's job now. Which raises a question the transcript is about to make sharp: if the test doesn't hold the run phase open, who does?

## Who ends the run phase

The Tester does — and *when* it does is the subtlest line in the testbench. Chapter 33's Tester puts four commands and then does something that looks like padding:

```rust,ignore
    let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
    for _ in 0..20 {
        bfm.clk().falling_edge().await;
    }
```

`put` returns when the FIFO accepts a command — not when the DUT has answered it. If the Tester dropped its objection right after its last `put`, the run phase would end with commands still in the pipeline and results still in flight, and the scoreboard would check fewer results than it saw commands. Note carefully what that failure looks like: *nothing*. The zip in the scoreboard's `check` pairs what arrived; it cannot miss what never came. The testbench does not fail — it silently checks less, which is worse. So the Tester holds its objection for a flush: the Python testbench waits ten clocks, and this one waits twenty, because the multiply is the last operation sent and the slowest to come back.

The monitors and the Driver, meanwhile, are infinite loops that never object — responders, in Chapter 31's terms. When the Tester's guard drops, the phase ends around them, and then `extract`, `check`, and `report` walk the tree. That ordering is not free — it is a property the framework has to guarantee. Race the objection against the run phase as a whole and the consensus would take the *components* down with it: `check` would never run, and the test would pass with its scoreboard never executing. So the rule is the one Chapter 24 stated: each component races the objection event individually, the tree outlives the race, and the post-run phases always get their walk. A test that can pass with its checker dead is the UVM's oldest trap, in any language.

```text
# Figure 4: Testbench 6.0 running

      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running AluTest (1/1)  [ch34-connections-testbench-6.0/src/ch34_connections_testbench_6_0.rs:346]
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    240.00ns INFO     [AluTest.env.scoreboard]: Covered all operations
    240.00ns INFO     [AluTest.env.coverage]: coverage saw 4 of 4 ops
    240.00ns INFO     AluTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** AluTest                                      PASS         240.00      **
******************************************************************************
REGRESSION: PASS
```

All the scoreboard lines carry timestamp 240ns — after the flush, in the `check` phase, where Chapter 33 put the comparison. Four commands driven, four results checked against predictions, coverage complete, and two components reporting on the same command stream without either knowing about the other.

One paragraph on the scoreboard, because Chapter 32 promised it here: it subscribes to **two** streams with two `SubscribePort`s and two `Subscriber` impls — `CmdLog` for commands, `ResultLog` for results — and no macros anywhere. This is the multiple-analysis-input problem that SystemVerilog needs the `uvm_analysis_imp_decl` macros for, because a class gets one `write` method; and it works identically when both streams carry the *same* type, which is precisely the case those macros exist to solve. The `uvm_tlm_analysis_fifo`s that would sit inside a UVM scoreboard are absent for Chapter 32's reason: the subscriber owns its storage, and these two own a `Vec` each.

## Summary

Testbench 6.0 assembles Chapters 31 through 33 into one machine: a stimulus lane with back-pressure, two broadcast lanes without it, and seven connections in one `connect` method sharing a single idiom. The test shrinks to construction — build the BFM, file it, build the env — and the objection becomes the load-bearing end-of-test mechanism, held by the Tester through a twenty-clock flush because a scoreboard that zips streams cannot complain about results that never arrived. The two-stream scoreboard cashes Chapter 32's promise: one impl per stream, no macros, no buffer FIFOs.

The structure is now complete, and it never changes again — every remaining testbench in this book, including the shipped one, wires this same shape. What changes next is the *data*: the command tuple has been `(u8, u8, Ops)` long enough, and Chapter 35 gives transactions the treatment the UVM gives `uvm_object`.
