`timescale 1ns / 1ps
// Testbench for counter — xezim --wave writes counter.vcd
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
        $dumpflush;
        $finish;
    end

endmodule
