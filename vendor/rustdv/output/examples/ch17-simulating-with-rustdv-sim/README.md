# Chapter 17: Simulating with rustdv-sim — figure map

The DUT is the SystemVerilog counter (Figure 1, `sim-common/hdl/counter.sv`).
Run the chapter with:

```
sim-common/run_sim.sh ch17_simulating_with_rustdv_sim counter \
    sim-common/hdl/timescale.v sim-common/hdl/counter.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | A SystemVerilog counter | `sim-common/hdl/counter.sv` |
| 2 | A typo'd signal name is an Err, not a surprise | `src/ch17_simulating_with_rustdv_sim.rs` (`name_lookup`) |
| 3 | get_int() ports to one line of unwrap_or | `src/ch17_simulating_with_rustdv_sim.rs` (`get_int`) |
| 4 | Starting the clock, lowering reset | `src/ch17_simulating_with_rustdv_sim.rs` (`no_count`, first half) |
| 5 | Wait for five clocks and check the output | `src/ch17_simulating_with_rustdv_sim.rs` (`no_count`, second half) |
| 6 | Testing that the counter counts | `src/ch17_simulating_with_rustdv_sim.rs` (`three_count`) |
| 7 | Forgetting the await is now a compiler warning | `src/ch17_simulating_with_rustdv_sim.rs` (`oops`) — the build emits the warning shown in the book, on purpose |

All 4 tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).
