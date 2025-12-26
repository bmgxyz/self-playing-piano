all: coil-winder.hex

coil-winder.hex: Cargo.lock Cargo.toml src/*
	cargo build --release
	avr-objcopy -O ihex target/avr-none/release/coil-winder.elf coil-winder.hex

flash: coil-winder.hex
	avrdude -p m328p -c arduino -P /dev/ttyUSB0 -b 57600 -v -U flash:w:coil-winder.hex

clean:
	cargo clean
	rm -f *.hex
