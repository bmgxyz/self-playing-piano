#include <avr/io.h>
#include <avr/interrupt.h>
#include <stdbool.h>

#define TWI_ACK_NEXT  { TWCR = (1 << TWINT) | (1 << TWEN) | (1 << TWEA) | (1 << TWIE); }
#define TWI_NACK_NEXT { TWCR = (1 << TWINT) | (1 << TWEN) | (1 << TWIE); }

extern void pwm(uint8_t *table_b, uint8_t *table_d);

// libc zeros these arrays automatically
static uint8_t table_b[127];
static uint8_t table_d[127];

enum state {
    idle, first, second, third
};

static volatile enum state s = idle;
static volatile uint16_t message = 0;
static volatile uint8_t key_idx = 0;
static volatile uint8_t key_vel = 0;

const uint8_t TWI_ADDR_PREFIX = 0x50;

void update_pwm(uint8_t key_idx, uint8_t key_vel) {
    uint8_t table_idx = key_idx < 6 ? key_idx : key_idx - 6;
    uint8_t *table_to_update = key_idx < 6 ? table_b : table_d;
    for (int i = 0; i < 127; i++) {
        if (i < key_vel) {
            // set bit
            table_to_update[i] |= 1 << table_idx;
        } else {
            // clear bit
            table_to_update[i] &= ~(1 << table_idx);
        }
    }
}

bool valid_checksum(uint16_t message) {
    return __builtin_popcount(message) % 6 == 0;
}

ISR(TWI_vect) {
    // quoted comments for each case come from Table 21-5 in the datasheet
    switch (TWSR & 0xf8) {
        // "Own SLA+W has been received; ACK has been returned"
        case 0x60:
            message = 0;
            key_idx = 0;
            key_vel = 0;
            s = first;
            TWI_ACK_NEXT
            break;
        // "Previously addressed with own SLA+W; data has been received; ACK has been returned"
        case 0x80:
            switch (s) {
                case first:
                    message = ((uint16_t)TWDR) << 8;
                    s = second;
                    TWI_ACK_NEXT
                    break;
                case second:
                    message |= ((uint16_t)TWDR);
                    s = third;
                    key_idx = (uint8_t)(message >> 12);
                    key_vel = (uint8_t)((message >> 5) % (1 << 7));
                    if (key_idx < 11 && valid_checksum(message)) {
                        update_pwm(key_idx, key_vel);
                        TWI_ACK_NEXT
                    } else {
                        TWI_NACK_NEXT
                    }
                    break;
                default:
                    s = idle;
                    TWI_ACK_NEXT
                    break;
            }
            break;
        default:
            message = 0;
            key_idx = 0;
            key_vel = 0;
            s = idle;
            TWI_ACK_NEXT
            break;
    }
}

int main(void) {
    DDRB = 0b00111111; // PB0 through PB5
    DDRD = 0b00111111; // PD0 through PD4, plus PD5 for status LED

    // I2C address is equal to the prefix together with the low nibble of PINC
    // shifted left by one to avoid TWGCE in TWAR
    TWAR = (TWI_ADDR_PREFIX | (PINC & 0b111)) << 1;

    // configure I2C for slave receiver mode with interrupts
    TWCR = (1 << TWEA) | (1 << TWEN) | (1 << TWIE);
    TWI_ACK_NEXT

    // enable global interrupts
    sei();

    while (1) {
        if (PINC & (1 << PC3)) {
            PORTD |= 1 << PD5;
            pwm(table_b, table_d);
            PORTD &= ~(1 << PD5);
        }
    }
}
