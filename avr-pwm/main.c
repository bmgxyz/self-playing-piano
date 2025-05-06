#include <avr/io.h>

extern void pwm(uint8_t *table_b, uint8_t *table_d);

// libc zeros these arrays automatically
static uint8_t table_b[127];
static uint8_t table_d[127];

enum state {
    idle, start, key, velocity
};

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

int main(void) {
    DDRB = 0b00111111;
    DDRD = 0b00011111;
    // I2C address is equal to the low nibble of PINC shifted left by one
    TWAR = (PINC & 0b1111) << 1;
    // configure I2C for slave receiver mode
    TWCR = (1 << TWEA) | (1 << TWEN);

    enum state s = idle;
    uint8_t key_idx = 0;
    uint8_t key_vel = 0;

    while (1) {
        // poll I2C
        if (TWCR & (1 << TWINT)) {
            switch (TWSR & 0xf8) {
                case 0x60:
                    s = start;
                    break;
                case 0x80:
                    if (s == start) {
                        if (TWDR > 11) {
                            key_idx = 0;
                            s = idle;
                        } else {
                            key_idx = TWDR;
                            s = key;
                        }
                    } else if (s == key) {
                        if (TWDR > 127) {
                            key_idx = 0;
                            key_vel = 0;
                            s = idle;
                        } else {
                            s = velocity;
                            key_vel = TWDR;
                        }
                    }
                    break;
                case 0xa0:
                    if (s == velocity) {
                        update_pwm(key_idx, key_vel);
                    }
                    key_idx = 0;
                    key_vel = 0;
                    s = idle;
                    break;
            }
            TWCR |= (1 << TWINT) | (1 << TWEN) | (1 << TWEA);
        }
        pwm(table_b, table_d);
    }
}
