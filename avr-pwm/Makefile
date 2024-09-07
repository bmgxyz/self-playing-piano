avr-pwm: main.S
	./unroll.py main.S > .tmp.S
	avr-gcc -mmcu=atmega328p -nostartfiles -o avr-pwm .tmp.S

flash: avr-pwm
	avrdude -p atmega328p -c arduino -P /dev/ttyUSB0 -b 57600 -D -U flash:w:avr-pwm

clean:
	rm .tmp.S avr-pwm
