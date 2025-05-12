#include <avr/io.h>
#include <stdbool.h>

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
    // wait for flashing before claiming USART pins
    UCSR0B = 0;
    UCSR0C = 0;
    DDRB = 0b00111111; // PB0 through PB5
    DDRD = 0b00011111; // PD0 through PD4
    // I2C address is equal to the low nibble of PINC shifted left by one
    TWAR = (PINC & 0b1111) << 1;
    // configure I2C for slave receiver mode
    TWCR = (1 << TWEA) | (1 << TWEN);

    enum state s = idle;
    uint8_t key_idx = 0;
    uint8_t key_vel = 0;
    bool i2c_active = false;

    while (1) {
        // poll I2C
        if (TWCR & (1 << TWINT)) {
            i2c_active = true;
            switch (TWSR & 0xf8) {
                case 0x60:
                    s = start;
                    break;
                case 0x80:
                    uint8_t data = TWDR;
                    if (s == start) {
                        if (data > 11) {
                            key_idx = 0;
                            s = idle;
                        } else {
                            key_idx = data;
                            s = key;
                        }
                    } else if (s == key) {
                        if (data > 127) {
                            key_idx = 0;
                            key_vel = 0;
                            s = idle;
                        } else {
                            s = velocity;
                            key_vel = data;
                        }
                    }
                    break;
                case 0xa0:
                    if (s == velocity) {
                        update_pwm(key_idx, key_vel);
                    }
                    PORTB &= ~(1 << PB0);
                    key_idx = 0;
                    key_vel = 0;
                    s = idle;
                    break;
            }
            if (s == idle) {
                i2c_active = false;
            }
            TWCR |= (1 << TWINT) | (1 << TWEN) | (1 << TWEA);
        }
        if (!i2c_active) {
            pwm(table_b, table_d);
        }
    }
}
