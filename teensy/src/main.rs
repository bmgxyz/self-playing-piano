#![no_std]
#![no_main]

use index::KeyIndex;
use log::Logger;
use midi::handle_midi_packet;
use state::KeyState;
use teensy4_bsp::{
    self as bsp,
    board::Lpi2c1,
    hal::{
        ccm::{clock_gate, lpi2c_clk},
        usbd::{BusAdapter, EndpointMemory, EndpointState, Speed},
    },
};
use teensy4_panic as _;

use bsp::board;
use core::fmt::Write;
use pwm::{KeyPwm, PwmManager};
use usb_device::{
    class_prelude::*,
    device::{StringDescriptors, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_midi::{UsbMidiClass, UsbMidiPacketReader};
use usbd_serial::SerialPort;

mod index;
mod log;
mod midi;
mod pwm;
mod state;

static EP_MEM: EndpointMemory<1024> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();

const NUM_KEYS: usize = 88;

#[bsp::rt::entry]
fn main() -> ! {
    let mut logger = Logger::new();
    let instances = board::instances();
    let board::Resources {
        mut ccm,
        gpt1,
        gpt2,
        mut gpio1,
        lpi2c1,
        pins,
        usb,
        ..
    } = board::t41(instances);

    clock_gate::lpi2c::<1>().set(&mut ccm, clock_gate::OFF);
    lpi2c_clk::set_selection(&mut ccm, lpi2c_clk::Selection::Oscillator);
    lpi2c_clk::set_divider(&mut ccm, lpi2c_clk::MIN_DIVIDER);
    clock_gate::lpi2c::<1>().set(&mut ccm, clock_gate::ON);

    let i2c: Lpi2c1 = board::lpi2c(lpi2c1, pins.p19, pins.p18, board::Lpi2cClockSpeed::KHz100);

    let control = gpio1.output(pins.p17);

    clock_gate::usb().set(&mut ccm, clock_gate::ON);
    let bus_adapter = BusAdapter::with_speed(usb, &EP_MEM, &EP_STATE, Speed::LowFull);
    let bus_allocator = UsbBusAllocator::new(bus_adapter);
    let mut midi = UsbMidiClass::new(&bus_allocator, 1, 1).unwrap();
    let mut serial =
        SerialPort::new_with_interface_names(&bus_allocator, Some("CDC Control"), Some("CDC Data"));
    let mut device = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0xffff, 0x0001))
        .strings(&[StringDescriptors::new(LangID::EN).product("Bothoven")])
        .unwrap()
        .build();
    loop {
        if device.poll(&mut [&mut midi, &mut serial]) {
            let state = device.state();
            if state == UsbDeviceState::Configured {
                break;
            }
        }
    }
    device.bus().configure();

    let mut pwm_manager = PwmManager::new(i2c, gpt1, gpt2, control);

    debug!(logger, "Bothoven ready");

    loop {
        device.poll(&mut [&mut midi, &mut serial]);

        let mut buffer = [0; 64];

        if let Ok(size) = midi.read(&mut buffer) {
            let buffer_reader = UsbMidiPacketReader::new(&buffer, size);
            for packet in buffer_reader.into_iter().flatten() {
                handle_midi_packet(&mut logger, packet, &mut pwm_manager);
            }
        }

        pwm_manager.tick(&mut logger);

        if let Ok(len) = serial.write(&logger.logs_to_write) {
            logger.advance(len);
        }
    }
}
