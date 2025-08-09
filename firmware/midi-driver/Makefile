all: midi-driver.hex \
	 transition-diagram.svg

midi-driver.hex: Cargo.lock Cargo.toml src/*
	cargo build
	arm-none-eabihf-objcopy -O ihex target/thumbv7em-none-eabihf/debug/midi-driver midi-driver.hex

transition-diagram.svg: transition-diagram.gv
	dot transition-diagram.gv -Tsvg > transition-diagram.svg

flash: midi-driver.hex
	teensy-loader-cli --mcu=TEENSY41 -vsw midi-driver.hex

watch:
	cat /dev/ttyACM0

clean:
	cargo clean
	rm -f *.hex *.svg
