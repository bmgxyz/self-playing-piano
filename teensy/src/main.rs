#![no_std]
#![no_main]

use teensy4_bsp::{
    board,
    hal::{
        ccm::clock_gate,
        usbd::{BusAdapter, EndpointMemory, EndpointState, Speed},
    },
};
use teensy4_panic as _;

use core::fmt::Write;
use usb_device::{
    class_prelude::*,
    device::{StringDescriptors, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_midi::{UsbMidiClass, UsbMidiPacketReader};
use usbd_serial::SerialPort;

static EP_MEM: EndpointMemory<1024> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();

mod index;
mod log;
mod midi;
mod pwm;
mod state;

use index::KeyIndex;
use log::Logger;
use midi::handle_midi_packet;
use pwm::PwmManager;
use state::KeyState;

const NUM_KEYS: usize = 88;

#[rtic::app(device = teensy4_bsp, peripherals = false)]
mod app {
    use heapless::spsc::Queue;
    use usb_device::device::UsbDevice;
    use usbd_midi::UsbMidiEventPacket;

    use super::*;

    #[local]
    struct Local {
        pwm_manager: PwmManager,
        midi: UsbMidiClass<'static, BusAdapter>,
        serial: SerialPort<'static, BusAdapter>,
        device: UsbDevice<'static, BusAdapter>,
    }

    #[shared]
    struct Shared {
        logger: Logger,
        midi_packets: Queue<UsbMidiEventPacket, 16>,
    }

    #[init(local = [bus: Option<UsbBusAllocator<BusAdapter>> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut logger = Logger::new();
        let midi_packets: Queue<UsbMidiEventPacket, 16> = Queue::new();
        let instances = board::instances();
        let board::Resources {
            mut ccm,
            gpt1,
            gpio1,
            pins,
            usb,
            flexpwm4,
            ..
        } = board::t41(instances);

        clock_gate::usb().set(&mut ccm, clock_gate::ON);
        let bus_adapter = BusAdapter::with_speed(usb, &EP_MEM, &EP_STATE, Speed::LowFull);
        bus_adapter.set_interrupts(true);
        let bus_allocator = ctx.local.bus.insert(UsbBusAllocator::new(bus_adapter));
        let mut midi = UsbMidiClass::new(bus_allocator, 1, 1).unwrap();
        let mut serial = SerialPort::new_with_interface_names(
            bus_allocator,
            Some("CDC Control"),
            Some("CDC Data"),
        );
        let mut device = UsbDeviceBuilder::new(bus_allocator, UsbVidPid(0xffff, 0x0001))
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

        let pwm_manager = PwmManager::new(gpio1, pins, gpt1, flexpwm4);

        debug!(logger, "Bothoven ready");

        (
            Shared {
                logger,
                midi_packets,
            },
            Local {
                pwm_manager,
                midi,
                serial,
                device,
            },
        )
    }

    #[idle(shared = [logger, midi_packets], local = [pwm_manager])]
    fn idle(mut ctx: idle::Context) -> ! {
        let idle::LocalResources { pwm_manager, .. } = ctx.local;
        loop {
            ctx.shared.logger.lock(|logger| {
                ctx.shared.midi_packets.lock(|midi_packets| {
                    while let Some(packet) = midi_packets.dequeue() {
                        debug!(logger, "Dequeued packet: {:?}", packet);
                        handle_midi_packet(logger, packet, pwm_manager);
                    }
                });
                pwm_manager.tick(logger);
            });
        }
    }

    #[task(binds = USB_OTG1, shared = [logger, midi_packets], local = [midi, serial, device])]
    fn usb(mut ctx: usb::Context) {
        let usb::LocalResources {
            midi,
            serial,
            device,
            ..
        } = ctx.local;

        if !device.poll(&mut [midi, serial]) {
            return;
        }

        let mut buffer = [0; 64];

        if let Ok(size) = midi.read(&mut buffer) {
            let buffer_reader = UsbMidiPacketReader::new(&buffer, size);
            for packet in buffer_reader.into_iter().flatten() {
                ctx.shared.midi_packets.lock(|midi_packets| {
                    ctx.shared.logger.lock(|logger| {
                        debug!(logger, "Received MIDI packet: {:?}", packet);
                        if let Err(_e) = midi_packets.enqueue(packet) {
                            warn!(logger, "Failed to enqueue MIDI packet, dropping")
                        }
                    });
                });
            }
        }

        ctx.shared.logger.lock(|logger| {
            if let Ok(len) = serial.write(&logger.logs_to_write) {
                logger.advance(len);
            }
        });
    }
}
