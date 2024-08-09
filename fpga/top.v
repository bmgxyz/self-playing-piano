module top(
    input clk_cnt,
    input clk_sr,
    input d,
    input latch,
    output reg[3:0] leds,
    output reg[11:0] out
);
    piano p (
        .clk_cnt(clk_cnt),
        .clk_sr(clk_sr),
        .d(d),
        .latch(latch),
        .leds(leds),
        .out(out)
    );
endmodule
