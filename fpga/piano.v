module vel_ram (
    input clk,
    input r_en,
    input w_en,
    input[6:0] r_addr,
    input[6:0] w_addr,
    output reg[11:0] r_data,
    input[11:0] w_data
);
    integer idx;
    reg[11:0] mem[0:127];

    initial begin
        r_data = 0;
        for (idx = 0; idx < 128; idx = idx + 1) begin
            mem[idx] = 0;
        end
    end

    always @(posedge clk) begin
        if (w_en) mem[w_addr] <= w_data;
        if (r_en) r_data <= mem[r_addr];
    end
endmodule

module piano(
    input clk_cnt,
    input clk_sr,
    input d,
    input latch,
    output reg[3:0] leds,
    output reg[NUM_KEYS-1:0] out
);
    // There are 88 piano keys, plus one extra pin for the sustain pedal
    localparam NUM_KEYS = 12;

    localparam STATE_IDLE = 0;
    localparam STATE_WRITING = 1;
    reg state;

    integer idx;

    reg[6:0] cnt;
    reg[13:0] sr;
    reg r_en, w_en;
    reg[NUM_KEYS-1:0] w_data;
    reg[6:0] vel;
    reg[6:0] start_cnt;
    reg[NUM_KEYS-1:0] key_oh;
    reg[7:0] w_cnt;
    wire[NUM_KEYS-1:0] out_cur;

    vel_ram ram (
        .clk(clk_cnt),
        .r_en(r_en),
        .w_en(w_en),
        .r_addr(cnt),
        .w_addr(cnt),
        .r_data(out_cur),
        .w_data(w_data)
    );

    initial begin
        cnt = 0;
	    sr = 0;
        r_en = 1;
        w_en = 0;
        w_data = 0;
        out = 0;
        vel = 0;
        start_cnt = 0;
        key_oh = 0;
        state = STATE_IDLE;
        w_cnt = 0;
    end

    always @(state) leds[0] <= state == 0;
    always @(d) leds[1] <= d;
    always @(latch) leds[2] <= latch;
    always @(clk_cnt) leds[3] <= clk_cnt;

    // When counter clock ticks, set all output pins and increment counter
    always @(posedge clk_cnt) begin
        out <= out_cur;
        cnt <= cnt + 1;
        // When the latch is enabled, write the lower half of the shift
        // register value to the appropriate location in velocity RAM
        case (state)
            STATE_IDLE: begin
                if (latch) begin
                    state <= STATE_WRITING;
                    w_en <= 0;
                    vel <= sr[6:0];
                    key_oh <= 1 << sr[13:7];
                    start_cnt <= cnt;
                end
            end
            STATE_WRITING: begin
                if (cnt < vel || vel == 127) begin
                    w_en <= 1;
                    // Clear the bit because we want the key to be on here (negative logic)
                    w_data <= ~key_oh & out_cur;
                end else if (cnt >= vel) begin
                    w_en <= 1;
                    // Set the bit because we want the key to be off here (negative logic)
                    w_data <= key_oh | out_cur;
                end
                if (w_cnt >= 136) begin
                    w_en <= 0;
                    w_cnt <= 0;
                    state <= STATE_IDLE;
                end else w_cnt <= w_cnt + 1;
            end
        endcase
    end

    // When shift register clock ticks, shift in a new bit
    always @(posedge clk_sr) sr <= (sr << 1) | d;
endmodule
