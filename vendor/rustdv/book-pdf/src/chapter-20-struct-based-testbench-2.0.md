# Chapter 20: Struct-Based Testbench: 2.0

Version 1.0 mixed everything in one loop; Chapter 19 pulled the pins out into the BFM. Version 2.0 takes the step both earlier books took next: break the *testbench* functionality — stimulus, checking, coverage — into separate pieces with names, so different tests reuse them instead of copying them. In SystemVerilog and Python alike, those pieces were classes related by inheritance. Here they are structs and a trait, and this chapter is where Part I's inheritance-versus-traits argument (Chapter 10) stops being an argument and starts being a testbench.

> **In the UVM...** we drew a UML diagram. A `BaseTester` defined `execute()` and left the operand supply undefined — a pure virtual method in SystemVerilog, an "ask forgiveness" abstract method in Python — while `RandomTester` and `MaxTester` extended it, each supplying one small method. A `Scoreboard` gathered commands and results into lists through two tasks and checked them all at the end. And the test wired everything together with *the tester's class itself* as the variation point.

## The Tester trait

Both earlier books opened this step with a UML diagram; the Rust structure diagram is smaller, because there is no base class — only a trait and two implementors:

```text
# Figure 1: Tester structure

              trait Tester
        fn get_operands(&mut self)   <- required (the "abstract method")
        async fn execute(&mut self)  <- provided (the shared behavior)
              /            \
     RandomTester         MaxTester
     random bytes         0xFF, 0xFF
```

The Python design's heart was a hole: `BaseTester.execute()` called `self.get_operands()`, a method that did not exist, trusting a subclass to fill it in — and trusting every future teammate to notice. Rust expresses "shared behavior with one deliberate hole" as a trait with a default method, and the hole is *declared*:

```rust
// Figure 2: Common behavior across all testers

#[allow(async_fn_in_trait)]
pub trait Tester {
    fn get_operands(&mut self) -> (u8, u8);

    async fn execute(&mut self, bfm: &TinyAluBfm) {
        for op in Ops::ALL {
            let (aa, bb) = self.get_operands();
            bfm.send_op(aa, bb, op).await;
        }
        // send two dummy operations to allow
        // the last real operation to complete
        bfm.send_op(0, 0, Ops::Add).await;
        bfm.send_op(0, 0, Ops::Add).await;
    }
}
```

Chapter 10's default-method machinery, load-bearing at last: `execute` is written once, in the trait, and calls `self.get_operands()` — which every implementor is *required* to provide, checked at compile time. A tester that forgets `get_operands` does not run and fail; it does not compile. What Python called an abstract base class and enforced by runtime `AttributeError`, Rust calls a required method and enforces before the simulator starts.²

Two smaller notes. `execute` takes the BFM as a parameter rather than conjuring the singleton — Chapter 19 removed the singleton, so dependencies now arrive through arguments, a small habit that Chapter 25 grows into a methodology. And the two dummy operations at the end are an old trick, preserved: they keep the pipeline moving so the last real operation completes before the test stops generating stimulus. (Version 2.0 is honest, not elegant. Hold that thought.)

The concrete testers are as small as their Python originals:

```rust
// Figure 3: RandomTester overrides get_operands()

pub struct RandomTester {
    pub rng: Rng,
}

impl Tester for RandomTester {
    fn get_operands(&mut self) -> (u8, u8) {
        (self.rng.u8(), self.rng.u8())
    }
}
```

```rust
// Figure 4: MaxTester overrides get_operands()

pub struct MaxTester;

impl Tester for MaxTester {
    fn get_operands(&mut self) -> (u8, u8) {
        (0xFF, 0xFF)
    }
}
```

`RandomTester` carries its own `Rng` — the seeded generator is state, and state lives in the struct, visibly. `MaxTester` has no state at all, so it is a *unit struct*, a type with no fields whose only job is to select an implementation. One thing does one thing.

> ² Python presents an abstract method's absence as a feature of dynamic typing, and it is — the same feature, viewed from the other side, that let a typo'd override silently define a *new* method instead of overriding anything. SystemVerilog's `pure virtual` closes the first door, at the price of joining a class hierarchy. The trait closes both doors with one key.

## The Scoreboard

Same definition as ever: a scoreboard gathers data from the DUT, predicts results, and compares. Same structure, too — two gathering tasks feeding storage, and a check function that runs after stimulus ends. The Rust version's storage types deserve a hard look, because they are this chapter's honest pain:

```rust
// Figure 5: Initializing the Scoreboard

pub struct Scoreboard {
    bfm: Rc<TinyAluBfm>,
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

impl Scoreboard {
    pub fn new(bfm: Rc<TinyAluBfm>) -> Scoreboard {
        Scoreboard {
            bfm,
            cmds: Rc::new(RefCell::new(Vec::new())),
            results: Rc::new(RefCell::new(Vec::new())),
            cvg: HashSet::new(),
        }
    }
```

`Rc<RefCell<Vec<...>>>`. In Python, the scoreboard's spawned tasks appended to `self.cmds` and nobody thought twice — every Python reference is shared and mutable. Rust makes us say it: the lists are mutated by *spawned tasks* while also being read later by *the scoreboard* — shared ownership (`Rc`) of mutable state (`RefCell`), Chapter 13's escape hatch, deployed exactly as advertised. It works, and the compiler holds us to single-writer discipline at runtime. But feel the friction, because the friction is the lesson: hand-wiring shared mutable lists between tasks is work the methodology should be doing for you. Chapter 32 gives this exact pattern a home — the subscriber that owns its storage, with the framework's `RustdvShared` marking the one sanctioned seam — and version 6.0 shows it managed instead of hand-rolled.

```rust
// Figure 6: The Scoreboard's data-gathering tasks

    pub fn start_tasks(&self) {
        let (bfm, cmds) = (self.bfm.clone(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmd",
        );
        let (bfm, results) = (self.bfm.clone(), self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_result",
        );
    }
```

Python's `get_cmd`/`get_result` coroutines plus `start_tasks`, fused: each task clones its handles (the Chapter 19 pattern) and loops forever appending. Commands and results correlate by arrival order, as in the Python original — order is the contract, which works precisely as long as the DUT completes operations in order. (It does. The TinyALU remains obligingly tiny.)

```rust
// Figure 7: The check_results() phase

    pub fn check_results(&mut self) -> bool {
        let mut passed = true;
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                passed = false;
                log::error(&format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }

// Figure 8: The Scoreboard checks functional coverage

        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            log::error("Functional coverage error: missed operations");
            passed = false;
        } else {
            log::info("Covered all operations");
        }
        passed
    }
}
```

Why does the scoreboard bother with coverage when the tester loops over all ops? The classic answer stands: the scoreboard must work with *any* tester, including future ones that don't. The scoreboard checks what happened, not what the stimulus promised.

## execute_test(): one wiring for all tests

```rust
// Figure 9: The execute_test coroutine starts the tasks

async fn execute_test(ctx: &RustdvCtx, tester: &mut impl Tester) -> Result<bool, TestError> {
    // The RTL self-clocks (tinyalu.sv); the BFM only waits on edges.
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    let mut scoreboard = Scoreboard::new(bfm.clone());
    bfm.reset().await;
    bfm.start_tasks();
    scoreboard.start_tasks();

// Figure 10: Execute the tester

    tester.execute(&bfm).await;
    let passed = scoreboard.check_results();
    Ok(passed)
}
```

Python's `execute_test(tester_class)` took a *class* and instantiated it — runtime dynamism at its most Pythonic. Rust's takes `&mut impl Tester`: any type implementing the trait, resolved at compile time (Chapter 11's generics — this function is monomorphized once per tester type, at zero runtime cost). The caller constructs the tester; `execute_test` neither knows nor cares which one it got. That is the same test-writing ergonomics, minus the ability to pass a class that turns out not to be a tester at all.

## The tests

```rust
// Figure 11: The tests launch execute_test with a tester

#[rustdv::test]
async fn random_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Random operands
    let mut tester = RandomTester { rng: ctx.rng() };
    let passed = execute_test(&ctx, &mut tester).await?;
    if passed {
        Ok(())
    } else {
        Err(TestError::from("random_test saw failing comparisons"))
    }
}
```

```rust
// Figure 12: The max test differs only in its tester

#[rustdv::test]
async fn max_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Maximum operands
    let mut tester = MaxTester;
    let passed = execute_test(&ctx, &mut tester).await?;
    if passed {
        Ok(())
    } else {
        Err(TestError::from("max_test saw failing comparisons"))
    }
}
```

Two tests, differing in one constructed value — the components did all the work, exactly as this testbench has always been designed. The transcript, both tests in one regression:

```text
# Figure 13: Two tests, one testbench
--
    150.00ns INFO     PASSED: c1 Add 67 = 0128
    150.00ns INFO     PASSED: 5e And 0b = 000a
    150.00ns INFO     PASSED: b9 Xor 80 = 0039
    150.00ns INFO     PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     Covered all operations
    150.00ns INFO     random_test PASSED
    300.00ns INFO     PASSED: ff Add ff = 01fe
    300.00ns INFO     PASSED: ff And ff = 00ff
    300.00ns INFO     PASSED: ff Xor ff = 0000
    300.00ns INFO     PASSED: ff Mul ff = fe01
    150.00ns INFO     Covered all operations
    300.00ns INFO     max_test PASSED
```

Note the timestamps: unlike Chapters 18 and 19, where PASSED lines trickled out as results arrived, all four comparisons print at once — the scoreboard checks *after* the run, batch-style. Chapter 24 will give that timing a name (`check`, a lifecycle phase) and a guarantee (it runs after the run phase ends). Note also `ff Xor ff = 0000` and `ff Mul ff = fe01` doing what max-operand tests exist to do: probing the corners where a lazier predictor would have wrapped, zeroed, or overflowed.

## Summary

Testbench 2.0 broke the test's remaining jobs into named pieces. The `Tester` trait holds the shared `execute` loop and declares the `get_operands` hole; `RandomTester` and `MaxTester` fill it in a line apiece, with the compiler enforcing what Python's abstract base class could only hope for. The `Scoreboard` gathers commands and results through spawned tasks into shared lists and batch-checks them with prediction and coverage — paying openly, in `Rc<RefCell<Vec>>` plumbing, for the shared mutable state that Python's references hid. `execute_test` wires it all once, generically over `impl Tester`, so each new test is a few lines constructing a different tester.

And with that, Part II's promise is kept: language, executor, tasks, queues, signals, and two working testbench architectures. What we have *not* kept doing is pretending the wiring scales — the by-hand construction in `execute_test`, the shared-list scoreboard, the tester passed around by argument. Making structure, configuration, and communication into reusable methodology is precisely the UVM's business, and Chapters 22 through 39 rebuild it in Rust. But between here and there stands one load-bearing chapter of language: macros — how `#[rustdv::test]` has been finding our tests all along, and how code that writes code replaces decorators and metaclasses. Chapter 21.
