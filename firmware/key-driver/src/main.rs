#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use arduino_hal::{
    delay_us,
    port::{mode::Output, Pin, PinOps},
};
use panic_halt as _;

use common::{Command, ModuleKeyIndex, ModuleKeyState, TWI_BASE_ADDR};

/// "Previously addressed with own SLA+W; data has been received; ACK has been returned"
///
/// Listed as `TWSR == 0x80` in the 328P manual (Table 21-5)
const TWI_SLA_W_DATA_RECV: u8 = 0x10;

// TODO check for keys that may be stuck due to missed or absent commands and release them
// automatically

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let module_addr = TWI_BASE_ADDR
        | (pins.a3.is_high() as u8) << 3
        | (pins.a2.is_high() as u8) << 2
        | (pins.a1.is_high() as u8) << 1
        | pins.a0.is_high() as u8;
    let twi = dp.TWI;
    twi.twar.write(|w| w.twa().variant(module_addr));
    twi.twcr.write(|w| w.twea().set_bit().twen().set_bit());

    let mut key_0 = pins.d2.into_output();
    let mut key_1 = pins.d3.into_output();
    let mut key_2 = pins.d4.into_output();
    let mut key_3 = pins.d5.into_output();
    let mut key_4 = pins.d6.into_output();
    let mut key_5 = pins.d7.into_output();
    let mut key_6 = pins.d8.into_output();
    let mut key_7 = pins.d9.into_output();

    let mut led = pins.d13.into_output();
    let listen = pins.d12.into_pull_up_input();

    let mut key_states = [ModuleKeyState::Holding; 8];

    led.set_high();

    loop {
        if listen.is_low() {
            led.set_low();
            if twi.twsr.read().tws().bits() == TWI_SLA_W_DATA_RECV {
                if let Ok(command) = twi.twdr.read().bits().try_into() {
                    let Command { key_idx, new_state } = command;
                    key_states[<ModuleKeyIndex as Into<usize>>::into(key_idx)] = new_state;
                }
                twi.twcr
                    .write(|w| w.twint().set_bit().twen().set_bit().twea().set_bit());
            }
            led.set_high();
        } else {
            // rising(&mut key_0, &key_states[0]);
            // rising(&mut key_1, &key_states[1]);
            // rising(&mut key_2, &key_states[2]);
            // rising(&mut key_3, &key_states[3]);
            // rising(&mut key_4, &key_states[4]);
            // rising(&mut key_5, &key_states[5]);
            // rising(&mut key_6, &key_states[6]);
            // rising(&mut key_7, &key_states[7]);
            // delay_us(9);
            // falling(&mut key_0, &key_states[0]);
            // falling(&mut key_1, &key_states[1]);
            // falling(&mut key_2, &key_states[2]);
            // falling(&mut key_3, &key_states[3]);
            // falling(&mut key_4, &key_states[4]);
            // falling(&mut key_5, &key_states[5]);
            // falling(&mut key_6, &key_states[6]);
            // falling(&mut key_7, &key_states[7]);
            // delay_us(41);

            rising(&mut key_0, &key_states[0]);
            delay_us(6);
            rising(&mut key_1, &key_states[1]);
            delay_us(3);
            falling(&mut key_0, &key_states[0]);
            delay_us(3);
            rising(&mut key_2, &key_states[2]);
            delay_us(3);
            falling(&mut key_1, &key_states[1]);
            delay_us(3);
            rising(&mut key_3, &key_states[3]);
            delay_us(3);
            falling(&mut key_2, &key_states[2]);
            delay_us(3);
            rising(&mut key_4, &key_states[4]);
            delay_us(3);
            falling(&mut key_3, &key_states[3]);
            delay_us(3);
            rising(&mut key_5, &key_states[5]);
            delay_us(3);
            falling(&mut key_4, &key_states[4]);
            delay_us(3);
            rising(&mut key_6, &key_states[6]);
            delay_us(3);
            falling(&mut key_5, &key_states[5]);
            delay_us(3);
            rising(&mut key_7, &key_states[7]);
            delay_us(3);
            falling(&mut key_6, &key_states[6]);
            delay_us(3);
            falling(&mut key_7, &key_states[7]);
        }
    }
}

fn rising<P: PinOps>(pin: &mut Pin<Output, P>, state: &ModuleKeyState) {
    match state {
        ModuleKeyState::Off => pin.set_low(),
        ModuleKeyState::Holding => pin.set_high(),
        ModuleKeyState::Pressing => pin.set_high(),
    }
}

fn falling<P: PinOps>(pin: &mut Pin<Output, P>, state: &ModuleKeyState) {
    match state {
        ModuleKeyState::Off => pin.set_low(),
        ModuleKeyState::Holding => pin.set_low(),
        ModuleKeyState::Pressing => pin.set_high(),
    }
}
