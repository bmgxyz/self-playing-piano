avr-pwm: main.c pwm.S
	avr-gcc -mmcu=atmega328p -Os -o avr-pwm main.c pwm.S

flash: avr-pwm
	avrdude -p atmega328p -c arduino -P /dev/ttyUSB0 -b 57600 -D -U flash:w:avr-pwm

clean:
	rm avr-pwm
