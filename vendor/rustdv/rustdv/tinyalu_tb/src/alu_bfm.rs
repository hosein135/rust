//! The BFM: owns typed DUT handles, runs driver/monitor loops as spawned
//! tasks, exposes queue-fed async methods (design-doc §7.2; port of the
//! book's TinyAluBfm chapter).
//!
//! Protocol (book "Basic testbench 1.0"): drive A/B/op and raise `start`
//! at a falling edge; hold until `done`; drop `start`. ADD/AND/XOR take
//! one cycle, MUL three.

use rustdv::prelude::*;

use crate::alu_item::{AluCommand, AluResult, Ops};

pub struct TinyAluBfm {
    clk: LogicHandle,
    reset_n: LogicHandle,
    start: LogicHandle,
    done: LogicHandle,
    a: LogicHandle,
    b: LogicHandle,
    op: LogicHandle,
    result: LogicHandle,
    driver_q: Queue<AluCommand>,
    cmd_q: Queue<AluCommand>,
    result_q: Queue<AluResult>,
}

/// The BFM lives in the ConfigDb, and a ConfigDb value must be `Debug` so
/// `ConfigDb::dump()` can name it. Hand-written, because a field-by-field dump
/// of eight signal handles and three queues would be noise: whoever reads a
/// config dump wants to know *which object* is filed there, not what is inside
/// it.
impl std::fmt::Debug for TinyAluBfm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TinyAluBfm")
    }
}

impl TinyAluBfm {
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
            driver_q: Queue::unbounded(),
            cmd_q: Queue::unbounded(),
            result_q: Queue::unbounded(),
        })
    }

    pub fn clk(&self) -> &LogicHandle {
        &self.clk
    }

    /// Reset the DUT (port of the book's bfm.reset()).
    pub async fn reset(&self) {
        self.reset_n.set_u64(0);
        self.start.set_u64(0);
        self.a.set_u64(0);
        self.b.set_u64(0);
        self.op.set_u64(0);
        for _ in 0..5 {
            self.clk.falling_edge().await;
        }
        self.reset_n.set_u64(1);
        self.clk.falling_edge().await;
        // Drop anything monitors saw during reset.
        while self.cmd_q.try_get().is_some() {}
        while self.result_q.try_get().is_some() {}
    }

    /// Queue one operation for the driver loop (book: send_op).
    pub async fn send_op(&self, cmd: AluCommand) {
        self.driver_q.put(cmd).await;
    }

    /// Monitor stream of observed commands (book: get_cmd).
    pub async fn get_cmd(&self) -> AluCommand {
        self.cmd_q.get().await
    }

    /// Monitor stream of observed results (book: get_result).
    pub async fn get_result(&self) -> AluResult {
        self.result_q.get().await
    }

    /// Wait until the driver queue is drained and the handshake is idle,
    /// plus one edge for monitors to flush — the test's end-of-stimulus
    /// drain before check.
    pub async fn wait_idle(&self) {
        // Two consecutive idle edges: a command popped from the queue but
        // not yet driven (start rises at the *next* ReadWrite phase) must
        // not fool us into declaring the bus quiet.
        let mut idle_edges = 0;
        while idle_edges < 2 {
            self.clk.falling_edge().await;
            if self.driver_q.is_empty() && self.start.is_low() && self.done.is_low() {
                idle_edges += 1;
            } else {
                idle_edges = 0;
            }
        }
        self.clk.falling_edge().await;
    }

    /// Spawn the three free-running BFM loops (book: start_bfm).
    pub fn start_tasks(&self) {
        // driver loop (book: driver_bfm state machine)
        {
            let (clk, start, done) = (self.clk, self.start, self.done);
            let (a, b, op) = (self.a, self.b, self.op);
            let q = self.driver_q.clone();
            spawn_named(
                async move {
                    loop {
                        clk.falling_edge().await;
                        let st = start
                            .get_binstr()
                            .expect("start signal did not return a VPI vector value");
                        let dn = done
                            .get_binstr()
                            .expect("done signal did not return a VPI vector value");
                        if st == "0" && dn == "0" {
                            if let Some(cmd) = q.try_get() {
                                a.set_u64(cmd.a as u64);
                                b.set_u64(cmd.b as u64);
                                op.set_u64(cmd.op.as_u64());
                                start.set_u64(1);
                            }
                        } else if st == "1" && dn == "1" {
                            start.set_u64(0);
                        }
                    }
                },
                "bfm.driver",
            );
        }

        // command monitor loop (book: cmd_mon_bfm — detect start 0→1)
        {
            let (clk, start, a, b, op) = (self.clk, self.start, self.a, self.b, self.op);
            let q = self.cmd_q.clone();
            spawn_named(
                async move {
                    let mut prev_start = false;
                    loop {
                        clk.falling_edge().await;
                        let st = start.is_high();
                        if st && !prev_start {
                            let (Ok(av), Ok(bv), Ok(opv)) =
                                (a.get_u64(), b.get_u64(), op.get_u64())
                            else {
                                prev_start = st;
                                continue; // x/z during reset: skip
                            };
                            if let Some(ops) = Ops::from_u64(opv) {
                                let _ = q.try_put(AluCommand {
                                    a: av as u8,
                                    b: bv as u8,
                                    op: ops,
                                });
                            }
                        }
                        prev_start = st;
                    }
                },
                "bfm.cmd_mon",
            );
        }

        // result monitor loop (book: result_mon_bfm — capture at done)
        {
            let (clk, start, done, result) = (self.clk, self.start, self.done, self.result);
            let q = self.result_q.clone();
            spawn_named(
                async move {
                    loop {
                        clk.falling_edge().await;
                        if start.is_high()
                            && done.is_high()
                            && let Ok(r) = result.get_u64()
                        {
                            let _ = q.try_put(AluResult { result: r as u16 });
                        }
                    }
                },
                "bfm.result_mon",
            );
        }
    }
}
