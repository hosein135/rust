//! Scaffold templates for new files and samples.

pub fn module_template(name: &str) -> String {
    let safe = sanitize_ident(name);
    format!(
        r#"`timescale 1ns / 1ps
// {safe}.v — RTL module
module {safe} (
    input  wire clk,
    input  wire rst_n,
    input  wire [7:0] data_in,
    output reg  [7:0] data_out
);

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            data_out <= 8'h00;
        else
            data_out <= data_in;
    end

endmodule
"#
    )
}

/// Testbench scaffold with `$dumpfile` / `$dumpvars` so xezim `--wave` writes a VCD.
pub fn testbench_template(name: &str) -> String {
    let safe = sanitize_ident(name);
    let vcd = format!("{safe}.vcd");
    format!(
        r#"`timescale 1ns / 1ps
// {safe}.v — testbench (xezim writes {vcd} when you click Run)
module {safe};

    // Instantiate the DUT here, then drive clocks / stimulus below.

    initial begin
        $dumpfile("{vcd}");
        $dumpvars(0, {safe});
        #100;
        $dumpflush;
        $finish;
    end

endmodule
"#
    )
}

pub fn rust_module_template(name: &str) -> String {
    let safe = sanitize_ident(name);
    let ty = pascal_case(&safe);
    format!(
        r#"//! TxHDL unit `{ty}`.
//! A design file is Rust. Press Run (F5) from a Cargo package that has a `*_tb.rs`.
//! The bundled sample is `samples/full_adder`: `src/adder.rs` and `src/full_adder_tb.rs`.
use txhdl::comp::{{Clock, DefaultClock, In, Out, Reg, Unit}};
use txhdl::types::Bit;
use txhdl::{{Trace, lower, with}};

/// One register, written on the rising edge from `d`.
#[derive(Trace, Default)]
pub struct {ty} {{
    /// Latched copy of `d`. Named apart from the `q` port.
    pub q_reg: Reg<Bit>,
}}

#[lower]
impl Unit for {ty} {{
    async fn run(&mut self, d: In<Bit>, q: Out<Bit>) {{
        loop {{
            DefaultClock::rising().await;
            let next = d.get();
            with!(self <= {{ q_reg: next }});
            q.set(next);
        }}
    }}
}}
"#
    )
}

/// rustdv testbench scaffold. The file name should end in `_tb.rs`.
pub fn rust_testbench_template(name: &str) -> String {
    let safe = sanitize_ident(name.trim_end_matches("_tb").trim_end_matches("_test"));
    let ty = pascal_case(&safe);
    format!(
        r#"//! rustdv testbench for `{ty}`.
//! Run (F5) compiles this package and runs the component below.
use rustdv::prelude::*;

/// Directed test. Replace the body with reads and checks of your TxHDL unit.
#[rustdv::test(name = "{safe}_test")]
#[derive(Component, Default)]
pub struct {ty}Test;

impl Component for {ty}Test {{
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {{
        let _guard = ctx.raise_objection("stimulus");
        let _ = ctx;
        // Drive the TxHDL unit here and return Err(TestError::new(...)) on a mismatch.
        Ok(())
    }}
}}
"#
    )
}

fn pascal_case(snake: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for ch in snake.chars() {
        if ch == '_' {
            up = true;
        } else if up {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "Unit".into()
    } else {
        out
    }
}

pub fn is_rust_filename(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".rs")
}

pub fn is_testbench_filename(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let stem = n.rsplit_once('.').map(|(s, _)| s).unwrap_or(&n);
    stem.ends_with("_tb")
        || stem.starts_with("tb_")
        || stem.ends_with("_testbench")
        || stem.ends_with("_test")
        || stem.contains("testbench")
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if i == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch);
        } else if ch == '-' || ch == ' ' {
            out.push('_');
        }
    }
    if out.is_empty() {
        "design".into()
    } else {
        out
    }
}
