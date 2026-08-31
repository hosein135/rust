//! Scaffold templates for modules and testbenches.

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

pub fn testbench_template(dut: &str) -> String {
    let safe = sanitize_ident(dut);
    let tb = format!("{safe}_tb");
    format!(
        r#"`timescale 1ns / 1ps
// {tb}.v — testbench for {safe}
module {tb};

    reg        clk;
    reg        rst_n;
    reg  [7:0] data_in;
    wire [7:0] data_out;

    {safe} uut (
        .clk     (clk),
        .rst_n   (rst_n),
        .data_in (data_in),
        .data_out(data_out)
    );

    // 100 MHz clock
    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        $dumpfile("{safe}.vcd");
        $dumpvars(0, {tb});

        rst_n   = 1'b0;
        data_in = 8'h00;
        #20;
        rst_n = 1'b1;

        data_in = 8'hA5;
        #20;
        data_in = 8'h3C;
        #20;
        data_in = 8'hFF;
        #40;

        $display("TB done. data_out = %02h", data_out);
        $finish;
    end

endmodule
"#
    )
}

pub fn counter_example() -> (&'static str, &'static str, &'static str, &'static str) {
    (
        "counter.v",
        r#"`timescale 1ns / 1ps
// 4-bit up counter
module counter (
    input  wire       clk,
    input  wire       rst_n,
    input  wire       en,
    output reg  [3:0] q
);

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            q <= 4'b0000;
        else if (en)
            q <= q + 4'b0001;
    end

endmodule
"#,
        "counter_tb.v",
        r#"`timescale 1ns / 1ps
// Testbench for counter
module counter_tb;

    reg        clk;
    reg        rst_n;
    reg        en;
    wire [3:0] q;

    counter uut (
        .clk   (clk),
        .rst_n (rst_n),
        .en    (en),
        .q     (q)
    );

    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        $dumpfile("counter.vcd");
        $dumpvars(0, counter_tb);

        rst_n = 1'b0;
        en    = 1'b0;
        #20;
        rst_n = 1'b1;
        en    = 1'b1;

        repeat (20) @(posedge clk);

        $display("Final count = %0d", q);
        $finish;
    end

endmodule
"#,
    )
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
