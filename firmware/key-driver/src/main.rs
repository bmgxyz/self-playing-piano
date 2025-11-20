#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

use core::{
    arch::asm,
    cell::{OnceCell, RefCell, UnsafeCell},
    sync::atomic::{AtomicBool, Ordering},
};

use arduino_hal::{
    delay_us,
    port::{mode::Output, Pin},
    prelude::*,
};
use atmega_hal::{
    clock::MHz16,
    pac::USART0,
    port::{mode::Input, PD0, PD1},
    usart::{Event, Usart},
};
use avr_device::interrupt::{self, Mutex};
use common::{
    Action, ModuleIndex, Schedule, ACK_RESPONSE, NAK_RESPONSE, SERIAL_BAUD_RATE, SERIAL_BUF_SIZE,
};
use heapless::Vec;
use panic_halt as _;

type Serial = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>;

static SERIAL: Mutex<OnceCell<UnsafeCell<Serial>>> = Mutex::new(OnceCell::new());
static SERIAL_BUF: Mutex<RefCell<Vec<u8, SERIAL_BUF_SIZE>>> = Mutex::new(RefCell::new(Vec::new()));
static MODULE_INDEX: Mutex<OnceCell<ModuleIndex>> = Mutex::new(OnceCell::new());
static SCHEDULE: Mutex<RefCell<Schedule>> = Mutex::new(RefCell::new(Schedule::empty()));
static SCHEDULE_UPDATE_FLAG: AtomicBool = AtomicBool::new(true);

#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    if let Some(b) = read_serial() {
        interrupt::free(|cs| {
            let mut buf = SERIAL_BUF.borrow(cs).borrow_mut();
            if buf.is_full() {
                buf.remove(0);
            }
            let _ = buf.push(b);
            if buf.len() >= 2 && buf.ends_with(b"\r\n") {
                buf.pop();
                buf.pop();
                match Schedule::deserialize(buf.as_slice()) {
                    Ok(new_schedule) => {
                        let mut schedule = SCHEDULE.borrow(cs).borrow_mut();
                        *schedule = new_schedule;
                        SCHEDULE_UPDATE_FLAG.store(true, Ordering::SeqCst);
                        write_serial(&ACK_RESPONSE);
                    }
                    Err(_) => write_serial(&NAK_RESPONSE),
                }
                buf.clear();
            }
        })
    }
}

fn set_serial(serial: Serial) -> Result<(), Serial> {
    interrupt::free(|cs| match SERIAL.borrow(cs).set(UnsafeCell::new(serial)) {
        Ok(()) => Ok(()),
        Err(u) => Err(u.into_inner()),
    })
}

fn read_serial() -> Option<u8> {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL.borrow(cs).get() {
            if let Some(s) = unsafe { serial.get().as_mut() } {
                s.read().ok()
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn write_serial(message: &[u8]) {
    interrupt::free(|cs| {
        if let Some(serial) = SERIAL.borrow(cs).get() {
            if let Some(s) = unsafe { serial.get().as_mut() } {
                for byte in message {
                    s.write_byte(*byte);
                }
            }
        }
    })
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, SERIAL_BAUD_RATE);
    serial.listen(Event::RxComplete);
    let _ = set_serial(serial);
    unsafe { avr_device::interrupt::enable() };

    let addr_0 = pins.a0.into_pull_up_input();
    let addr_1 = pins.a1.into_pull_up_input();
    let addr_2 = pins.a2.into_pull_up_input();
    let module_index = match ModuleIndex::new(
        (addr_2.is_high() as u8) << 2 | (addr_1.is_high() as u8) << 1 | addr_0.is_high() as u8,
    ) {
        Some(idx) => idx,
        None => unreachable!(),
    };
    interrupt::free(|cs| {
        if MODULE_INDEX.borrow(cs).set(module_index).is_err() {
            unreachable!()
        }
    });

    // in addition to setting these pins as outputs, this prevents any future code from taking
    // ownership of the pins as well
    let _key_00 = pins.d2.into_output();
    let _key_01 = pins.d3.into_output();
    let _key_02 = pins.d4.into_output();
    let _key_03 = pins.d5.into_output();
    let _key_04 = pins.d6.into_output();
    let _key_05 = pins.d7.into_output();
    let _key_06 = pins.d8.into_output();
    let _key_07 = pins.d9.into_output();
    let _key_08 = pins.d10.into_output();
    let _key_09 = pins.d11.into_output();
    let _key_10 = pins.d12.into_output();
    let _key_11 = pins.d13.into_output();

    interrupt::free(|cs| {
        let mut schedule = SCHEDULE.borrow(cs).borrow_mut();
        *schedule = Schedule::all_off();
    });

    let mut schedule = Schedule::empty();

    loop {
        if SCHEDULE_UPDATE_FLAG.load(Ordering::SeqCst) {
            schedule = interrupt::free(|cs| SCHEDULE.borrow(cs).clone().into_inner());
            SCHEDULE_UPDATE_FLAG.store(false, Ordering::SeqCst);
        }
        for action in schedule.actions.iter() {
            match action {
                Action::Delay { duration_us } => delay_us((*duration_us).into()),
                Action::SetKeyPins { port_b, port_d } => unsafe {
                    asm!("out 0x05, {}", in(reg) *port_b);
                    asm!("out 0x0b, {}", in(reg) *port_d);
                },
            }
        }
    }
}
