# Interlude: The Complete TinyALU Testbench

> **In the UVM...** the destination was always the same summit: a TinyALU driven by sequences, checked by a scoreboard, measured by coverage, held together by the methodology — whether *The UVM Primer* built it in SystemVerilog or *Python for RTL Verification* built it in pyuvm, both ended there. This interlude shows you that summit in Rust — the complete, running rustdv testbench — *before* the climb. Nothing here is pseudocode: every line below is the shipped `tinyalu_tb` crate, and it runs on Icarus Verilog to `REGRESSION: PASS`.

Part I handed you fourteen chapters of language and kept saying they were load-bearing. This is the load. Read it the way you would walk through a finished house before studying the blueprints: do not try to understand it — try to *recognize* it. You know this testbench. You have built it in another language. The point of the next few pages is that when you squint, it is the tool you already own, spelled in the language you just learned — and the parts you cannot read yet each have a chapter with their name on it. The map of those chapters closes the interlude. Chapter 40 walks this same code with everything explained.

## The transactions

```rust
// Figure 1: The TinyALU transactions (tinyalu_tb/src/alu_item.rs)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}

/// Golden model (the scoreboard's predictor).
pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}
```

This figure you can read completely — it is Chapters 4, 6, 7, and 10 doing their jobs. A transaction is a plain struct; no `uvm_sequence_item` base class, and the jobs the base class did arrive as derives: `Clone` is `do_copy`, `PartialEq` is `do_compare`, `Debug` is the printable form. The predictor is a function and a `match`.

## The stimulus

```rust
// Figure 2: A sequence — stimulus as a program (tinyalu_tb/src/sequences.rs)

/// How the operands get filled, once the driver is committed.
trait Operands {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand);
}

/// Every operation, `n` times each — the walk all three sequences share.
async fn all_ops<S: Operands>(
    seq: &mut S,
    ctx: &mut SeqCtx<AluCommand, AluResult>,
    n: usize,
) -> Result<(), SeqError> {
    let mut rng = ctx.rng();
    for _ in 0..n {
        for op in Ops::ALL {
            let mut cmd = AluCommand { a: 0, b: 0, op };
            ctx.start_item(&mut cmd).await?;
            // Late generation: the driver is waiting, so decide now.
            seq.set_operands(&mut rng, &mut cmd);
            // Ownership moves to the driver here. A sequence that needed the
            // command afterward would clone it first; this one does not.
            ctx.finish_item(cmd).await?;
        }
    }
    Ok(())
}

/// Random operands across every operation, five times each.
#[derive(Default)]
pub struct RandomSeq;

impl Operands for RandomSeq {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = rng.u8();
        cmd.b = rng.u8();
    }
}

impl Sequence for RandomSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 5).await
    }
}
```

`start_item`, `finish_item`, a body that loops the operations: the sequence idiom you know, and the comment about ownership moving is Chapter 5 speaking. What `SeqCtx` and `Sequence` are, and why the rendezvous has two calls, is Chapter 36's whole subject.

## A component

```rust
// Figure 3: The driver (tinyalu_tb/src/components.rs)

#[derive(Component, Default)]
pub struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            bfm.send_op(item.payload().clone()).await;
            self.seq_item_port.item_done(None);
        }
    }
}
```

`get_next_item`, drive, `item_done` — `uvm_driver`'s loop, recognizable at a glance. Two things to merely notice, not yet understand: the driver takes *no constructor arguments* — the BFM arrives from something called the `ConfigDb`, by name — and its `run` returns a `Result`, with `?` doing what Chapter 9 taught.

## The scoreboard

```rust
// Figure 4: The scoreboard — two streams in, verdicts in check
//           (tinyalu_tb/src/components.rs)

#[derive(Default)]
struct CmdLog {
    cmds: Vec<AluCommand>,
}

impl Subscriber<AluCommand> for CmdLog {
    fn write(&mut self, cmd: &AluCommand) {
        self.cmds.push(cmd.clone());
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<AluResult>,
}

impl Subscriber<AluResult> for ResultLog {
    fn write(&mut self, res: &AluResult) {
        self.results.push(res.clone());
    }
}

#[derive(Component, Default)]
pub struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    #[port(subscribe)]
    result_in: SubscribePort<AluResult>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    compared: usize,
    mismatches: usize,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.cmd_log.clone());
        self.result_in.subscribe(self.result_log.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();

        for (cmd, actual) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
            let expected = predict(cmd);
            self.compared += 1;
            if expected != *actual {
                self.mismatches += 1;
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=FAIL"
                ));
                errors.error(format!(
                    "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                ));
            } else {
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=PASS"
                ));
            }
        }

        if cmd_log.cmds.len() != result_log.results.len() {
            errors.error(format!(
                "scoreboard: saw {} commands and {} results",
                cmd_log.cmds.len(),
                result_log.results.len()
            ));
        }
        if self.compared == 0 {
            errors.error("scoreboard: nothing was compared".to_string());
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}
```

A scoreboard that subscribes to two streams — commands and results — predicts with the figure-1 function, compares with `PartialEq`, and files its failures somewhere called a `CheckSink` inside a phase called `check`. Notice, without yet knowing why, that it holds a plain `Vec` for each stream, and that the last two error checks refuse to let "nothing arrived" look like "nothing failed." There are also two monitors publishing onto the buses the scoreboard reads, and a coverage collector counting ops as a second subscriber on the command stream — the same shapes, not reprinted here.

## The environment

```rust
// Figure 5: The environment — build creates, connect wires
//           (tinyalu_tb/src/env.rs)

#[derive(Component, Default)]
pub struct AluEnv {
    #[component]
    seqr: Sequencer<AluCommand, AluResult>,
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
    cmd_bus: AnalysisBus<AluCommand>,
    #[component]
    result_bus: AnalysisBus<AluResult>,
    is_active: bool,
    with_coverage: bool,
}

impl Component for AluEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        self.cmd_mon = CmdMonitor::create_comp();
        self.result_mon = ResultMonitor::create_comp();
        self.scoreboard = Scoreboard::create_comp();
        if self.with_coverage {
            self.coverage = Coverage::create_comp();
        }

        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        if self.is_active {
            self.seqr
                .seq_item_export()
                .connect(&self.driver, Driver::SEQ_ITEM_PORT);
        }

        self.cmd_bus
            .pub_export()
            .connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus
            .sub_export()
            .connect(&self.scoreboard, Scoreboard::CMD_IN);
        if self.with_coverage {
            self.cmd_bus
                .sub_export()
                .connect(&self.coverage, Coverage::CMD_IN);
        }

        self.result_bus
            .pub_export()
            .connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus
            .sub_export()
            .connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

Here is the whole UVM vocabulary on one page: a `build` phase creating children top-down, a `connect` phase wiring them bottom-up, components built through something called a factory (`create_comp()`), an `IS_ACTIVE` knob read from the ConfigDb that decides whether a driver exists at all, and one broadcast bus per observed stream. If you have written a `uvm_env`, every line has a shape you have seen — down to the passive env that simply does not build its driver.

## The tests

```rust
// Figure 6: Two tests, one testbench (tinyalu_tb/src/tinyalu_tb.rs)

#[derive(Component, Default)]
pub struct BaseTest {
    #[component]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");

        let seqr: Sequencer<alu_item::AluCommand, alu_item::AluResult> =
            ConfigDb::get(Some(ctx), "", "SEQR")?;

        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.wait_idle().await;

        ctx.info("sequence complete");
        Ok(())
    }
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, RandomSeq>();
        self.inner = BaseTest::new_comp();
    }
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    inner: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}
```

Two tests, and neither adds a component. Each names a different sequence for the factory to substitute and reuses everything else — the whole point of the methodology, visible in six lines of difference.

```text
# Figure 7: The testbench running

      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [rustdv/tinyalu_tb/src/tinyalu_tb.rs:77]
     70.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 296 }
     70.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     90.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 10 }
     90.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 94, b: 11, op: And }
    110.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 57 }
    110.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    130.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    160.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 19305 }
    180.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 150, op: Add }
    180.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 318 }
    200.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 97, b: 254, op: And }
    200.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 96 }
    220.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 192, b: 138, op: Xor }
    220.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 74 }
    240.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 59, op: Mul }
    270.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 9912 }
    290.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 340 }
    290.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 99, b: 241, op: Add }
    310.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 8 }
    310.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 238, b: 8, op: And }
    330.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 218 }
    330.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 70, b: 156, op: Xor }
    350.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 205, b: 172, op: Mul }
    380.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 35260 }
    400.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 159, b: 247, op: Add }
    400.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 406 }
    420.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 53, b: 171, op: And }
    420.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 33 }
    440.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 39, b: 138, op: Xor }
    440.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 173 }
    460.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 132, b: 186, op: Mul }
    490.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 24552 }
    510.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 137 }
    510.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 109, b: 28, op: Add }
    530.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 4 }
    530.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 23, b: 12, op: And }
    550.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 52 }
    550.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 245, b: 193, op: Xor }
    570.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 24, b: 60, op: Mul }
    600.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 1440 }
    630.00ns INFO     [RandomTest.inner]: sequence complete
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 193, b: 103, op: Add } out=AluResult { result: 296 } expected=AluResult { result: 296 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 94, b: 11, op: And } out=AluResult { result: 10 } expected=AluResult { result: 10 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 185, b: 128, op: Xor } out=AluResult { result: 57 } expected=AluResult { result: 57 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 165, b: 117, op: Mul } out=AluResult { result: 19305 } expected=AluResult { result: 19305 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 150, op: Add } out=AluResult { result: 318 } expected=AluResult { result: 318 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 97, b: 254, op: And } out=AluResult { result: 96 } expected=AluResult { result: 96 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 192, b: 138, op: Xor } out=AluResult { result: 74 } expected=AluResult { result: 74 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 59, op: Mul } out=AluResult { result: 9912 } expected=AluResult { result: 9912 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 99, b: 241, op: Add } out=AluResult { result: 340 } expected=AluResult { result: 340 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 238, b: 8, op: And } out=AluResult { result: 8 } expected=AluResult { result: 8 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 70, b: 156, op: Xor } out=AluResult { result: 218 } expected=AluResult { result: 218 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 205, b: 172, op: Mul } out=AluResult { result: 35260 } expected=AluResult { result: 35260 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 159, b: 247, op: Add } out=AluResult { result: 406 } expected=AluResult { result: 406 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 53, b: 171, op: And } out=AluResult { result: 33 } expected=AluResult { result: 33 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 39, b: 138, op: Xor } out=AluResult { result: 173 } expected=AluResult { result: 173 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 132, b: 186, op: Mul } out=AluResult { result: 24552 } expected=AluResult { result: 24552 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 109, b: 28, op: Add } out=AluResult { result: 137 } expected=AluResult { result: 137 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 23, b: 12, op: And } out=AluResult { result: 4 } expected=AluResult { result: 4 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 245, b: 193, op: Xor } out=AluResult { result: 52 } expected=AluResult { result: 52 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 24, b: 60, op: Mul } out=AluResult { result: 1440 } expected=AluResult { result: 1440 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: 20 compared, 0 mismatches
    630.00ns INFO     [RandomTest.inner.env.coverage]: coverage: Add=5 And=5 Mul=5 Xor=5
    630.00ns INFO     RandomTest PASSED
    630.00ns INFO     running MaxTest (2/2)  [rustdv/tinyalu_tb/src/tinyalu_tb.rs:92]
    700.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 510 }
    700.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    720.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 255 }
    720.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: And }
    740.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 0 }
    740.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Xor }
    760.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Mul }
    790.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 65025 }
    820.00ns INFO     [MaxTest.inner]: sequence complete
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Add } out=AluResult { result: 510 } expected=AluResult { result: 510 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: And } out=AluResult { result: 255 } expected=AluResult { result: 255 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Xor } out=AluResult { result: 0 } expected=AluResult { result: 0 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Mul } out=AluResult { result: 65025 } expected=AluResult { result: 65025 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: 4 compared, 0 mismatches
    820.00ns INFO     [MaxTest.inner.env.coverage]: coverage: Add=1 And=1 Mul=1 Xor=1
    820.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         630.00      **
** MaxTest                                      PASS         190.00      **
******************************************************************************
REGRESSION: PASS
```

Twenty-four operations driven, predicted, compared, and counted, with every log line stamped with the path of the component that wrote it — and the run ends in the summary table and `REGRESSION: PASS`.

## What you could already read, and where the rest is taught

Tally what Part I just let you read fluently: enums and `match` in the transactions and the predictor (Chapters 4, 7); ownership moving the command into `finish_item`, with `clone()` as the alternative (Chapter 5); `Rc` where one BFM is truly shared (Chapter 13); `Result` and `?` threading every fallible step (Chapter 9); traits standing in for base classes, derives doing `uvm_object`'s jobs (Chapter 10); `SeqItemPort<AluCommand, AluResult>` and the other typed plumbing (Chapter 11); `Vec` and `HashMap` holding what the subscribers keep (Chapter 8); and a crate you could build and unit-test with `cargo` (Chapter 14).

What you took on faith is exactly the rest of the book. The page after this one — the rustdv Toolkit — names every framework identifier you just squinted at. Then: `async`/`await` and the executor underneath every `run` (Chapter 15), tasks and queues (Chapter 16), the simulator connection and the BFM (Chapters 17–19), the macros behind `#[rustdv::test]` and `#[derive(Component)]` (Chapter 21), tests as components (Chapter 23), the nine phases and the growing tree (Chapter 24), the ConfigDb that delivered the BFM (Chapters 25, 27–28), the factory behind `create_comp` and `create_seq` (Chapter 29), ports, FIFOs and the connect idiom (Chapter 31), the analysis buses and why the scoreboard owns its own `Vec`s (Chapter 32), testbench 6.0 wiring this very architecture (Chapters 33–34), transactions in full (Chapter 35), and the sequencer handshake (Chapter 36) with its response machinery (Chapters 37–38) and virtual sequences (Chapter 39). Chapter 40 then returns here, to this exact crate, and walks it with nothing left on faith.

The climb starts on the next page. It is worth it: at the top, this testbench is yours.
