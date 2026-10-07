//! One-bit full adder, written as a TxHDL unit.
//!
//! Three single-bit inputs: `a`, `b`, and carry-in. Outputs are `sum` and
//! `cout`. The rustdv testbench is `full_adder_tb.rs`.
//!
//! The body is the subset `#[lower]` reads: wait for the rising edge, read
//! the inputs, drive the sum and carry registers, and drive the output
//! wires from those same values. A value that crosses the `await` is a
//! register; the outputs are wires, so a testbench sees the result in the
//! same cycle it was computed.

use txhdl::comp::trace::{self, Vcd};
use txhdl::comp::{signal, Clock, DefaultClock, In, Out, Reg, Running, Unit};
use txhdl::types::Bit;
use txhdl::{lower, with, Trace};

/// One-bit full adder. The registers are named apart from the ports: the
/// netlist declares each name once.
#[derive(Trace, Default)]
pub struct FullAdder {
    /// Sum, `a ^ b ^ cin`, latched on the rising edge.
    pub sum_q: Reg<Bit>,
    /// Carry out, majority of the three inputs, latched on the rising edge.
    pub cout_q: Reg<Bit>,
}

#[lower]
impl Unit for FullAdder {
    async fn run(
        &mut self,
        (a, b, cin): (In<Bit>, In<Bit>, In<Bit>),
        (sum, cout): (Out<Bit>, Out<Bit>),
    ) {
        loop {
            DefaultClock::rising().await;
            let s = a.get() ^ b.get() ^ cin.get();
            let c = (a.get() & b.get()) | (a.get() & cin.get()) | (b.get() & cin.get());
            with!(self <= { sum_q: s, cout_q: c });
            sum.set(s);
            cout.set(c);
        }
    }
}

/// Drive one input vector for one default-clock cycle and read the wires.
pub fn apply(a: bool, b: bool, cin: bool) -> (bool, bool) {
    let (a_drv, a_in) = signal::<Bit, DefaultClock>();
    let (b_drv, b_in) = signal::<Bit, DefaultClock>();
    let (cin_drv, cin_in) = signal::<Bit, DefaultClock>();
    let (sum_drv, sum_in) = signal::<Bit, DefaultClock>();
    let (cout_drv, cout_in) = signal::<Bit, DefaultClock>();
    let mut dut = FullAdder::default();
    let mut sim = Running::new(dut.run((a_in, b_in, cin_in), (sum_drv, cout_drv)));
    a_drv.set(a);
    b_drv.set(b);
    cin_drv.set(cin);
    sim.cycle();
    (sum_in.get().to_bool(), cout_in.get().to_bool())
}

/// Run all eight input vectors on one design and, when `vcd_path` is set,
/// write a VCD of the testbench wires and the unit's registers.
///
/// The testbench drives `a`, `b`, and `cin`. The unit drives `sum` and
/// `cout`. One clock cycle is applied per vector, so the waveform shows
/// the whole directed test.
pub fn directed(vcd_path: Option<&std::path::Path>) -> Result<(), String> {
    let (a_drv, a) = signal::<Bit, DefaultClock>();
    let (b_drv, b) = signal::<Bit, DefaultClock>();
    let (cin_drv, cin) = signal::<Bit, DefaultClock>();
    let (sum_drv, sum) = signal::<Bit, DefaultClock>();
    let (cout_drv, cout) = signal::<Bit, DefaultClock>();
    let mut dut = FullAdder::default();

    if let Some(path) = vcd_path {
        let file = std::fs::File::create(path)
            .map_err(|e| format!("create {}: {e}", path.display()))?;
        let mut wave = Vcd::new(std::io::BufWriter::new(file));
        wave.clock::<DefaultClock>();
        wave.add("a", &a);
        wave.add("b", &b);
        wave.add("cin", &cin);
        wave.add("full_adder", &dut);
        wave.add("sum", &sum);
        wave.add("cout", &cout);
        wave.start();
    }

    let mut sim = Running::new(dut.run((a, b, cin), (sum_drv, cout_drv)));
    let mut errors = Vec::new();
    for bits in 0u8..8 {
        let a_bit = bits & 1 != 0;
        let b_bit = bits & 2 != 0;
        let cin_bit = bits & 4 != 0;
        a_drv.set(a_bit);
        b_drv.set(b_bit);
        cin_drv.set(cin_bit);
        sim.cycle();
        let got_sum = sum.get().to_bool();
        let got_cout = cout.get().to_bool();
        let expect_sum = a_bit ^ b_bit ^ cin_bit;
        let expect_cout = (a_bit && b_bit) || (a_bit && cin_bit) || (b_bit && cin_bit);
        if got_sum != expect_sum || got_cout != expect_cout {
            errors.push(format!(
                "a={a_bit} b={b_bit} cin={cin_bit}: got sum={got_sum} cout={got_cout}, expected sum={expect_sum} cout={expect_cout}"
            ));
        }
    }

    if vcd_path.is_some() {
        trace::stop();
        if let Some(path) = vcd_path {
            trim_closing_sample(path);
            println!("VCD written: {}", path.display());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// `trace::stop` samples once more at `u64::MAX` so the file is flushed.
/// That timestamp would stretch the viewer to the end of time, so drop it.
fn trim_closing_sample(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let marker = format!("#{}\n", u64::MAX);
    if let Some(at) = text.rfind(&marker) {
        let _ = std::fs::write(path, &text[..at]);
    }
}
