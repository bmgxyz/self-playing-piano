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

    let addr_0 = pins.a0.into_pull_up_input();
    let addr_1 = pins.a1.into_pull_up_input();
    let addr_2 = pins.a2.into_pull_up_input();
    let module_addr = TWI_BASE_ADDR
        | (addr_2.is_high() as u8) << 2
        | (addr_1.is_high() as u8) << 1
        | addr_0.is_high() as u8;
    let twi = dp.TWI;
    twi.twar.write(|w| w.twa().variant(module_addr));
    twi.twcr.write(|w| w.twea().set_bit().twen().set_bit());

    let mut key_00 = pins.d2.into_output();
    let mut key_01 = pins.d3.into_output();
    let mut key_02 = pins.d4.into_output();
    let mut key_03 = pins.d5.into_output();
    let mut key_04 = pins.d6.into_output();
    let mut key_05 = pins.d7.into_output();
    let mut key_06 = pins.d8.into_output();
    let mut key_07 = pins.d9.into_output();
    let mut key_08 = pins.d10.into_output();
    let mut key_09 = pins.d11.into_output();
    let mut key_10 = pins.d12.into_output();

    let mut led = pins.d13.into_output();
    let listen = pins.a3.into_pull_up_input();

    let mut key_states = [ModuleKeyState::Off; 11];

    led.set_low();

    loop {
        if listen.is_low() {
            led.set_high();
            if twi.twsr.read().tws().bits() == TWI_SLA_W_DATA_RECV {
                if let Ok(command) = twi.twdr.read().bits().try_into() {
                    let Command { key_idx, new_state } = command;
                    key_states[<ModuleKeyIndex as Into<usize>>::into(key_idx)] = new_state;
                }
            }
            twi.twcr
                .write(|w| w.twint().set_bit().twen().set_bit().twea().set_bit());
            led.set_low();
        } else {
            rising(&mut key_00, &key_states[0]);
            rising(&mut key_01, &key_states[1]);
            rising(&mut key_02, &key_states[2]);
            rising(&mut key_03, &key_states[3]);
            rising(&mut key_04, &key_states[4]);
            rising(&mut key_05, &key_states[5]);
            rising(&mut key_06, &key_states[6]);
            rising(&mut key_07, &key_states[7]);
            rising(&mut key_08, &key_states[8]);
            rising(&mut key_09, &key_states[9]);
            rising(&mut key_10, &key_states[10]);
            delay_us(9);
            falling(&mut key_00, &key_states[0]);
            falling(&mut key_01, &key_states[1]);
            falling(&mut key_02, &key_states[2]);
            falling(&mut key_03, &key_states[3]);
            falling(&mut key_04, &key_states[4]);
            falling(&mut key_05, &key_states[5]);
            falling(&mut key_06, &key_states[6]);
            falling(&mut key_07, &key_states[7]);
            falling(&mut key_08, &key_states[8]);
            falling(&mut key_09, &key_states[9]);
            falling(&mut key_10, &key_states[10]);
            delay_us(41);
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
