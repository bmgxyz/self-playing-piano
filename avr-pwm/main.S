.equ pinb,  0x03
.equ ddrb,  0x04
.equ portb, 0x05
.equ pinc,  0x06
.equ ddrc,  0x07
.equ portc, 0x08
.equ pind,  0x09
.equ ddrd,  0x0a
.equ portd, 0x0b
.equ spcr,  0x2c
.equ spsr,  0x2d
.equ spdr,  0x2e
.equ spif,     7
.equ ss,       2

_start:
    ldi r16, 0b00010000 ; set all SPI and external crystal pins as input except PB4 (MISO)
    out ddrb, r16
    ldi r16, 0b01000000 ; disable interrupt, enable SPI as slave, MSB, rising, leading
    out spcr, r16
    ldi r16, 0b00111111 ; set all pins as output except PC6 (/RESET)
    out ddrc, r16
    ldi r16, 0b11111111 ; set all pins as output
    out ddrd, r16
    ldi ZH, 0x01        ; initialize PWM table offset
    ldi ZL, 0x00        ; fall through to clear_pwm_table_setup
clear_pwm_table_setup:
    ldi r16, 0x00       ; write zeros to the PWM table
    ldi r17, 0x02       ; when XH reaches this value, we know we've written the whole table
    ldi XH, 0x01        ; init X to the start of the PWM table
    ldi XL, 0x00        ; fall through to clear_pwm_table_loop
clear_pwm_table_loop:
    st X+, r16          ; write zeros
    cpse XH, r17        ; if we've written the whole table, run PWM
    rjmp clear_pwm_table_loop
    rjmp pwm

duty_cycle: ; repeat 128 times
    ld r16, X+
    out portc, r16
    ld r16, Y+
    out portd, r16        ; fall through to PWM reset
pwm:
    wdr                   ; reset watchdog timer
    ldi r16, 0            ; set all outputs low
    out portc, r16
    out portd, r16
    ldi XH, 0x01          ; set C base pointer to 0x100
    ldi XL, 0x00
    ldi YH, 0x01          ; set D base pointer to 0x180
    ldi YL, 0x80
    ldi ZH, 0x01          ; set PWM data table pointer to 0x100
    ldi ZL, 0x00          ; fall through to SPI check
spi:
    in r16, spsr          ; if there's an SPI byte waiting, then read it
    sbrc r16, spif
    rjmp spi_read
    in r16, pinb          ; if /SS is low, then keep waiting for a byte
    sbrs r16, ss
    rjmp spi
    rjmp duty_cycle

spi_read:
    in r16, spdr          ; store the byte in the PWM data table at the current pointer
    st Z+, r16
    rjmp spi
