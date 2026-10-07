# Chapter 33: Components in Testbench 6.0

Configuration, the factory, logging, ports, broadcasting — the toolbox is full, and testbench 6.0 spends it. The 6.0 principle, quoted from the Python book because it cannot be improved: each component "either creates data and writes it to a port or gets data from a port and processes it." One job each. This chapter refactors the components to that standard — and only defines them. They connect to FIFOs and buses, not to each other, so a chapter of definitions has nothing it can run; Chapter 34 wires them up and does the running for both.

> **In the UVM...** we split the testbench into a `BaseTester` that put command tuples into a `uvm_put_port`, a `Driver` that pulled from a `uvm_get_port` and drove the BFM, monitors publishing on analysis ports, a `Coverage` subscriber, and a `Scoreboard` buffering two `uvm_tlm_analysis_fifo`s for the check phase. The Python version had a flourish: one `Monitor` class taking a *method name* string, with `getattr` fetching `get_cmd` or `get_result` at runtime.

## Stimulus: the Tester and the Driver

```rust
// Chapter 33, Figure 1: The Tester puts commands into a FIFO
#[derive(Component, Default)]
struct Tester {
    #[port(put)]
    cmd_port: PutPort<Command>,
    rng: Option<Rng>,
}

impl Component for Tester {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.rng = Some(ctx.rng());
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");
        let rng = self.rng.as_mut().expect("build ran");
        for op in Ops::ALL {
            self.cmd_port.put((rng.u8(), rng.u8(), op)).await;
        }
        // `put` returns as soon as the FIFO takes the command, not when the
        // DUT has answered it — so dropping the objection here would end the
        // phase with commands still in the pipeline and results in flight, and
        // the scoreboard would silently check fewer results than it saw
        // commands. Hold the objection for a flush, as the Python testbench
        // does. It waits ten clocks; this waits twenty, because the multiply
        // is the last operation and takes the longest to come back.
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        for _ in 0..20 {
            bfm.clk().falling_edge().await;
        }
        Ok(())
    }
}
```

The Tester creates data and writes it to a port — its whole job, per the principle. `Command` is a type alias for the `(u8, u8, Ops)` tuple, still; transactions get their upgrade in Chapter 35. Two things to note. The Tester never touches the BFM to *drive* — it only borrows the clock for the flush at the end, and the comment above that loop is the most important one in the chapter: `put` returning means *accepted*, not *answered*, and an objection dropped too early does not fail the testbench — it silently checks less. Chapter 34 returns to this when the objection becomes the thing that ends the whole run.

```rust
// Chapter 33, Figure 2: The Driver gets commands and drives the BFM
#[derive(Component, Default)]
struct Driver {
    #[port(get)]
    cmd_port: GetPort<Command>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let (aa, bb, op) = self.cmd_port.get().await; // blocks until a command
            bfm.send_op(aa, bb, op).await;
        }
    }
}
```

The Driver is the mirror image: gets data from a port, processes it. It is a responder in Chapter 31's sense — an infinite loop, no objection, blocking on an empty FIFO until the Tester supplies work, running for exactly as long as anyone still objects. Neither component knows the other exists; both know only their ends of a FIFO that Chapter 34 will put between them.

## Observation: two monitors

```rust
// Chapter 33, Figure 3: The command monitor watches the bus and broadcasts
#[derive(Component, Default)]
struct CmdMonitor {
    #[port(publish)]
    ap: PublishPort<CmdTuple>,
}

impl Component for CmdMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let cmd = bfm.get_cmd().await;
            self.ap.write(&cmd);
        }
    }
}
```

```rust
// Chapter 33, Figure 4: The result monitor broadcasts results
#[derive(Component, Default)]
struct ResultMonitor {
    #[port(publish)]
    ap: PublishPort<u64>,
}

impl Component for ResultMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let result = bfm.get_result().await;
            self.ap.write(&result);
        }
    }
}
```

Four lines of body each: watch the BFM's queue, `write` what appears, forever. The Python book made one `Monitor` class serve both jobs by passing the BFM method's *name* as a string and fetching it with `getattr` at runtime — a lovely trick in a language with runtime attribute lookup, and not one Rust offers. Two small components, written out, do the same work; the types differ anyway (`CmdTuple` versus `u64`), so nothing is duplicated but the shape. Neither monitor knows who is listening — that is the point of publishing.

## Checking: the Scoreboard, on two streams

```rust
// Chapter 33, Figure 5: The Scoreboard subscribes to BOTH streams
#[derive(Default)]
struct CmdLog {
    cmds: Vec<CmdTuple>,
}

impl Subscriber<CmdTuple> for CmdLog {
    fn write(&mut self, cmd: &CmdTuple) {
        self.cmds.push(*cmd);
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<u64>,
}

impl Subscriber<u64> for ResultLog {
    fn write(&mut self, result: &u64) {
        self.results.push(*result);
    }
}

#[derive(Component, Default)]
struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<CmdTuple>,
    #[port(subscribe)]
    result_in: SubscribePort<u64>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    cvg: HashSet<Ops>,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_cmds = self.cmd_log.clone();
        self.cmd_in.subscribe(my_cmds);

        let my_results = self.result_log.clone();
        self.result_in.subscribe(my_results);
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();
        for (cmd, result) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("legal op");
            self.cvg.insert(op);
            let actual = *result as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                ctx.info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }

        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            ctx.info("Covered all operations");
        }
    }
}
```

Here is Chapter 32's pattern at full size: **two streams, two `SubscribePort`s, two `Subscriber` impls** — one `write` per subscriber, each stream landing in the `Vec` it chose to keep, no macros minted and no buffer FIFOs routed. Compare the Chapter 25 scoreboard this replaces: gone are the spawned collector tasks and their `Rc<RefCell>` lists — delivery is synchronous now, so the subscribers just push — and gone is any contact with the BFM at all. The scoreboard's inputs are *ports*. It would work unchanged against any DUT whose monitors publish these two types, which is what "single job, standard connections" buys.

The comparison itself is unchanged since 4.0: zip commands against results, predict, compare, tally coverage, and report failures to the `CheckSink` in the `check` phase.

## Coverage: a second subscriber

```rust
// Chapter 33, Figure 6: Coverage subscribes to the command stream only
#[derive(Default)]
struct OpsSeen {
    ops: HashSet<Ops>,
}

impl Subscriber<CmdTuple> for OpsSeen {
    fn write(&mut self, cmd: &CmdTuple) {
        if let Some(op) = Ops::from_u64(cmd.2) {
            self.ops.insert(op);
        }
    }
}

#[derive(Component, Default)]
struct Coverage {
    #[port(subscribe)]
    cmd_in: SubscribePort<CmdTuple>,
    seen: RustdvShared<OpsSeen>,
}

impl Component for Coverage {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_subscriber = self.seen.clone();
        self.cmd_in.subscribe(my_subscriber);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = self.seen.get();
        ctx.info(&format!("coverage saw {} of {} ops", seen.ops.len(), Ops::ALL.len()));
    }
}
```

A second subscriber on the command stream. The monitor does not know Coverage exists; the scoreboard does not either; adding it to the testbench will cost Chapter 34 exactly one `connect` line. That is the decoupling the analysis hub buys, demonstrated by a component whose entire footprint is one line of wiring.

## Summary

Six components, one job each, and not a single one holds a reference to another: the Tester puts, the Driver gets and drives, two monitors publish, and the Scoreboard and Coverage subscribe — the scoreboard on two streams with two `Subscriber` impls, the pattern that needs no `imp_decl` machinery and no analysis FIFOs. Every input and output is a declared port; the BFM arrives by name; the flush comment in the Tester is a debt the objection story pays next chapter. Nothing here can run, because nothing here is connected.

Chapter 34 builds the environment that introduces them all to each other — seven connect lines, one idiom — and runs testbench 6.0.
