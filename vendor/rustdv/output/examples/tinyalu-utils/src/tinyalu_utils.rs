//! The Rust `tinyalu_utils`: the shared resources every testbench version
//! from Chapter 19 on imports — `Ops`, `alu_prediction`, `get_int`, and
//! the `TinyAluBfm`. In Python this was a module reached through a
//! `sys.path` trick; in Rust it is an ordinary library crate the chapter
//! crates list in `[dependencies]` (Chapter 14).


use rustdv::prelude::*;

// D45: infrastructure only. The promoted testbench modules — `tb2`, `tb4`,
// `tb6`, `tb7`, `env7`, `bfm7`, `alu_item` — were removed from this crate.
// Every Python chapter imports only the BFM, `Ops` and `alu_prediction`
// and *re-shows* the rest, because those classes evolve: `BaseTester` is a
// plain class at 3.0 and a component at 4.0, and re-showing them is how
// the reader sees the change. Factoring them out made the thing the book
// teaches invisible.
//
// The files were kept in `src/` so each chapter could lift its copy back
// into the chapter file as it converted. Every chapter has now done so, and
// the files were deleted on 2026-08-05: uncompiled since D45, they had been
// carried through two framework renames by threads whose work no compiler
// ever checked, and they were the last callers of an analysis surface
// deleted under D116. Git has them if a chapter ever wants one back.

// Chapter 18, Figure 2: The operation enumeration
// Legal ops for the TinyALU
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

impl Ops {
    pub const ALL: [Ops; 4] = [Ops::Add, Ops::And, Ops::Xor, Ops::Mul];

    /// Port of `Ops(int)` conversion: `Ops::from_u64(2) == Some(Ops::And)`.
    pub fn from_u64(v: u64) -> Option<Ops> {
        match v {
            1 => Some(Ops::Add),
            2 => Some(Ops::And),
            3 => Some(Ops::Xor),
            4 => Some(Ops::Mul),
            _ => None,
        }
    }
}

// Chapter 18, Figure 3: The prediction function for the scoreboard
pub fn alu_prediction(a: u8, b: u8, op: Ops) -> u16 {
    // Rust model of the TinyALU
    let (a, b) = (a as u16, b as u16);
    match op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    }
}

// Chapter 17, Figure 3: get_int() ports to one line of unwrap_or
pub fn get_int(signal: &LogicHandle) -> u64 {
    // x or z becomes 0, as tinyalu_utils decided
    signal.get_u64().unwrap_or(0)
}

/// A command on the TinyALU input buses, as the monitors see it.
pub type CmdTuple = (u64, u64, u64);

// Chapter 19, Figure 2: The TinyAluBfm struct — one owner of the pins
pub struct TinyAluBfm {
    clk: LogicHandle,
    reset_n: LogicHandle,
    start: LogicHandle,
    done: LogicHandle,
    a: LogicHandle,
    b: LogicHandle,
    op: LogicHandle,
    result: LogicHandle,
    driver_queue: Queue<(u8, u8, Ops)>,
    cmd_mon_queue: Queue<CmdTuple>,
    result_mon_queue: Queue<u64>,
}

impl std::fmt::Debug for TinyAluBfm {
    /// `ConfigDb` requires `Debug` so that `ConfigDb::dump()` can show what a
    /// component would see (D68). A field-by-field dump of eight signal
    /// handles and three queues would be noise, so this prints the one thing
    /// a reader wants to know: which object it is.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TinyAluBfm")
    }
}

impl TinyAluBfm {
    // Chapter 19, Figure 3: Initializing the TinyAluBfm
    pub fn new(dut: &HierarchyHandle) -> Result<TinyAluBfm, HandleError> {
        Ok(TinyAluBfm {
            clk: dut.signal("clk")?,
            reset_n: dut.signal("reset_n")?,
            start: dut.signal("start")?,
            done: dut.signal("done")?,
            a: dut.signal("A")?,
            b: dut.signal("B")?,
            op: dut.signal("op")?,
            result: dut.signal("result")?,
            driver_queue: Queue::new(Some(1)),
            cmd_mon_queue: Queue::unbounded(),
            result_mon_queue: Queue::unbounded(),
        })
    }

    pub fn clk(&self) -> &LogicHandle {
        &self.clk
    }

    // Chapter 19, Figure 4: Centralizing the reset function
    pub async fn reset(&self) {
        self.clk.falling_edge().await;
        self.reset_n.set_u64(0);
        self.start.set_u64(0);
        self.a.set_u64(0);
        self.b.set_u64(0);
        self.op.set_u64(0);
        self.clk.falling_edge().await;
        self.reset_n.set_u64(1);
        self.clk.falling_edge().await;
    }

    // Chapter 19, Figure 5: Monitoring the result bus
    fn result_mon(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, done, result) = (self.clk, self.done, self.result);
        let queue = self.result_mon_queue.clone();
        async move {
            let mut prev_done = 0;
            loop {
                clk.falling_edge().await;
                let dn = get_int(&done);
                if prev_done == 0 && dn == 1 {
                    let _ = queue.try_put(get_int(&result));
                }
                prev_done = dn;
            }
        }
    }

    // Chapter 19, Figure 6: Monitoring the command signals
    fn cmd_mon(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, start, a, b, op) = (self.clk, self.start, self.a, self.b, self.op);
        let queue = self.cmd_mon_queue.clone();
        async move {
            let mut prev_start = 0;
            loop {
                clk.falling_edge().await;
                let st = get_int(&start);
                if st == 1 && prev_start == 0 {
                    let cmd_tuple = (get_int(&a), get_int(&b), get_int(&op));
                    let _ = queue.try_put(cmd_tuple);
                }
                prev_start = st;
            }
        }
    }

    // Chapter 19, Figure 7: Driving commands on the falling edge of clk
    fn cmd_driver(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, start, done) = (self.clk, self.start, self.done);
        let (a, b, op) = (self.a, self.b, self.op);
        let queue = self.driver_queue.clone();
        async move {
            start.set_u64(0);
            a.set_u64(0);
            b.set_u64(0);
            op.set_u64(0);
            loop {
                clk.falling_edge().await;
                let st = get_int(&start);
                let dn = get_int(&done);
                if st == 0 && dn == 0 {
                    // Chapter 19, Figure 8: Drive a command when the bus is idle
                    match queue.try_get() {
                        Some((aa, bb, opr)) => {
                            a.set_u64(aa as u64);
                            b.set_u64(bb as u64);
                            op.set_u64(opr as u64);
                            start.set_u64(1);
                        }
                        None => continue,
                    }
                } else if st == 1 {
                    // Chapter 19, Figure 9: If start is 1 check done
                    if dn == 1 {
                        start.set_u64(0);
                    }
                }
            }
        }
    }

    // Chapter 19, Figure 10: Start the BFM tasks
    pub fn start_tasks(&self) {
        spawn_named(self.cmd_driver(), "bfm.cmd_driver");
        spawn_named(self.cmd_mon(), "bfm.cmd_mon");
        spawn_named(self.result_mon(), "bfm.result_mon");
    }

    // Chapter 19, Figure 11: The get_cmd() coroutine returns the next command
    pub async fn get_cmd(&self) -> CmdTuple {
        self.cmd_mon_queue.get().await
    }

    // Chapter 19, Figure 12: The get_result() coroutine returns the next result
    pub async fn get_result(&self) -> u64 {
        self.result_mon_queue.get().await
    }

    // Chapter 19, Figure 13: send_op puts the command into the command Queue
    pub async fn send_op(&self, aa: u8, bb: u8, op: Ops) {
        self.driver_queue.put((aa, bb, op)).await;
    }
}
