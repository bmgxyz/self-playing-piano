all: controller.hex \
	 transition-diagram.svg

controller.hex: Cargo.lock Cargo.toml src/*
	cargo build
	arm-none-eabihf-objcopy -O ihex target/thumbv7em-none-eabihf/debug/controller controller.hex

transition-diagram.svg: transition-diagram.gv
	dot transition-diagram.gv -Tsvg > transition-diagram.svg

flash: controller.hex
	teensy-loader-cli --mcu=TEENSY41 -vsw controller.hex

watch:
	sudo cat /dev/ttyACM0

clean:
	cargo clean
	rm -f *.hex *.svg
