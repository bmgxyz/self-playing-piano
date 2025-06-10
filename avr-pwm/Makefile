avr-pwm: main.c pwm.S
	avr-gcc -mmcu=atmega328p -Os -o avr-pwm main.c pwm.S

flash: avr-pwm
	avrdude -p m328p -c arduino -P /dev/ttyUSB0 -b 19200 -U flash:w:avr-pwm

fuse:
	avrdude -c arduino -P /dev/ttyUSB0 -b 19200 -p m328p -U lfuse:w:0xFF:m -U hfuse:w:0xDE:m -U efuse:w:0xFD:m

clean:
	rm -f avr-pwm
