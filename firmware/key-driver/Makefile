all: key-driver.hex

key-driver.hex: Cargo.lock Cargo.toml src/*
	cargo build
	avr-objcopy -O ihex target/avr-none/debug/key-driver.elf key-driver.hex

flash: key-driver.hex
	avrdude -p m328p -c arduino -P /dev/ttyUSB0 -b 57600 -v -U flash:w:key-driver.hex

clean:
	cargo clean
	rm -f *.hex
