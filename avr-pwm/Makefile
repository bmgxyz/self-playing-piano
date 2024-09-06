avr-pwm: main.S
	./unroll.py main.S > .tmp.S
	avr-gcc -mmcu=atmega328p -nostartfiles -o avr-pwm .tmp.S

clean:
	rm .tmp.S avr-pwm
