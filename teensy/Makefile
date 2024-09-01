all: controller.hex \
	 transition-diagram.svg

controller.hex: Cargo.lock Cargo.toml src/*
	cargo build
	rust-objcopy -O ihex target/thumbv7em-none-eabihf/debug/controller controller.hex

transition-diagram.svg: transition-diagram.gv
	dot transition-diagram.gv -Tsvg > transition-diagram.svg

flash:
	sudo $(which teensy_loader_cli) --mcu=TEENSY41 -s -w controller.hex

clean:
	cargo clean
	rm *.hex
	rm *.svg
