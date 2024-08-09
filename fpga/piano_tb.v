`include "piano.v"

`timescale 100ns/1ns
`default_nettype none

`define PULSE(p) p = 1; #1; p = 0; #1;
`define PULSE_N(p,n) for (i = 0; i < n; i = i + 1) `PULSE(p);

module tb();
    reg clk_cnt;
    reg clk_sr;
    reg d;
    reg latch;
    wire[88:0] out;
    integer i;

    //piano p(
    //    .clk_cnt(clk_cnt),
    //    .clk_sr(clk_sr),
    //    .d(d),
    //    .latch(latch),
    //    .out(out)
    //);

    //always #1 clk_cnt = ~clk_cnt;

    //initial begin
    //    $dumpfile("top_tb.vcd");
    //    $dumpvars;

	//clk_cnt = 0;
    //    clk_sr = 0;
    //    d = 0;
    //    latch = 0;

    //    #1;

    //    d = 1;
    //    `PULSE(clk_sr);
    //    `PULSE(clk_sr);
    //    `PULSE(clk_sr);
    //    `PULSE(clk_sr);
    //    `PULSE(clk_sr);
    //    `PULSE(clk_sr);

    //    d = 0;
	//latch = 1;
	//#10;
	//latch = 0;

    //    #1000;

    //    $finish;
    //end

endmodule
