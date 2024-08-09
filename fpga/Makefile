run: piano_tb.v piano.v
	iverilog -o piano_tb.vvp piano_tb.v
	vvp piano_tb.vvp
plot: top_tb.vcd
	gtkwave top_tb.vcd
build: top.v piano.v
	yosys -p "synth_ice40 -top top -json top.json" top.v piano.v
	nextpnr-ice40 --hx1k --package tq144 --pcf $(PINMAP) --pcf-allow-unconstrained --asc top.asc --json top.json
	icepack top.asc top.bin
prog:
	iceprog -d i:0x0403:0x6010:0 top.bin
