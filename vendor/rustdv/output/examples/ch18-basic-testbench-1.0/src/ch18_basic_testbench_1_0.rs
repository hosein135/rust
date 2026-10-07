//! Chapter 18: Basic testbench 1.0 — one loop, straight through.
//!
//!     sim-common/run_sim.sh ch18_basic_testbench_1_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Testbench version 1.0 from the book, ported figure for figure.

use std::collections::HashSet;

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

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

fn get_int(signal: &LogicHandle) -> u64 {
    signal.get_u64().unwrap_or(0)
}

// Chapter 18, Figures 4–11: the whole testbench, one loop
#[rustdv::test]
async fn alu_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Chapter 18, Figure 4: The start of the TinyALU test. Reset the DUT
    let dut = ctx.dut();
    let mut rng = ctx.rng();
    // The RTL self-clocks (tinyalu.sv); the BFM only waits on edges (D42).
    let clk = dut.signal("clk")?;

    let mut passed = true;
    let mut cvg: HashSet<Ops> = HashSet::new(); // functional coverage

    let reset_n = dut.signal("reset_n")?;
    let start = dut.signal("start")?;
    clk.falling_edge().await;
    reset_n.set_u64(0);
    start.set_u64(0);
    clk.falling_edge().await;
    reset_n.set_u64(1);

    // Chapter 18, Figure 5: Creating one transaction for each operation
    let mut cmd_count = 1;
    let mut op_list: Vec<Ops> = Ops::ALL.to_vec();
    let num_ops = op_list.len();
    let (mut aa, mut bb) = (0u8, 0u8);
    let mut op = Ops::Add;
    while cmd_count <= num_ops {
        clk.falling_edge().await;
        let st = get_int(&start);
        let dn = get_int(&dut.signal("done")?);

        // Chapter 18, Figure 6: Creating a TinyALU command
        if st == 0 && dn == 0 {
            aa = rng.u8();
            bb = rng.u8();
            op = op_list.remove(0);
            cvg.insert(op);
            dut.signal("A")?.set_u64(aa as u64);
            dut.signal("B")?.set_u64(bb as u64);
            dut.signal("op")?.set_u64(op as u64);
            start.set_u64(1);
        }

        // Chapter 18, Figure 7: Erroring on a state that must never happen
        if st == 0 && dn == 1 {
            return Err(TestError::from("DUT Error: done set to 1 without start"));
        }

        // Chapter 18, Figure 8: If we are in an operation, continue
        if st == 1 && dn == 0 {
            continue;
        }

        // Chapter 18, Figure 9: The operation is complete
        if st == 1 && dn == 1 {
            start.set_u64(0);
            cmd_count += 1;
            let result = get_int(&dut.signal("result")?) as u16;

            // Chapter 18, Figure 10: Checking results against the prediction
            let pr = alu_prediction(aa, bb, op);
            if result == pr {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {result:04x}"));
            } else {
                log::error(&format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {result:04x} - predicted {pr:04x}"
                ));
                passed = false;
            }
        }
    }

    // Chapter 18, Figure 11: Checking functional coverage using a set
    let missed: HashSet<Ops> = Ops::ALL.iter().filter(|op| !cvg.contains(op)).copied().collect();
    if !missed.is_empty() {
        log::error(&format!("Functional coverage error. Missed: {missed:?}"));
        passed = false;
    } else {
        log::info("Covered all operations");
    }

    // Chapter 18, Figure 12: The final check relays pass/fail to rustdv
    if passed {
        Ok(())
    } else {
        Err(TestError::from("alu_test saw failing comparisons"))
    }
}
