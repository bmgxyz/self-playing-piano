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

mod fsm;
mod index;
mod log;
mod midi;
mod pwm;
mod state;

use fsm::FsmManager;
use log::Logger;
use midi::handle_midi_packet;
use pwm::PwmManager;
use state::KeyState;

const NUM_KEYS: usize = 88;

#[rtic::app(device = teensy4_bsp, peripherals = false)]
mod app {
    use cortex_m::{asm::delay, delay};
    use embedded_hal::digital::{OutputPin, StatefulOutputPin};
    use heapless::spsc::Queue;
    use teensy4_bsp::{
        hal::{
            flexpwm::{
                self, ClockSelect, Interrupts, LoadMode, PairOperation, Prescaler, Status,
                Submodule, ValueRegister,
            },
            gpio::Output,
            gpt::{ClockSource, Mode, OutputCompareRegister},
        },
        pins::tmm::P0,
    };
    use usb_device::device::UsbDevice;
    use usbd_midi::UsbMidiEventPacket;

    use crate::{index::ModuleIndex, pwm::PwmEdge};

    use super::*;

    #[local]
    struct Local {
        fsm_manager: FsmManager,
        midi: UsbMidiClass<'static, BusAdapter>,
        serial: SerialPort<'static, BusAdapter>,
        device: UsbDevice<'static, BusAdapter>,
        test_pin: Output<P0>,
        pwm_submodule: Submodule<4, 1>,
    }

    #[shared]
    struct Shared {
        logger: Logger,
        key_states: [KeyState; NUM_KEYS],
        modules_to_update: [bool; ModuleIndex::NUM_MODULES],
        pwm_manager: PwmManager,
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
            mut gpio1,
            mut gpio2,
            mut gpio4,
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

        let (mut pwm_control, (_, mut pwm_submodule, _, _)) = flexpwm4;

        pwm_submodule.set_debug_enable(true);
        pwm_submodule.set_wait_enable(true);
        pwm_submodule.set_clock_select(flexpwm::ClockSelect::Ipg);
        pwm_submodule.set_prescaler(Prescaler::Prescaler1);
        pwm_submodule.set_pair_operation(flexpwm::PairOperation::Independent);
        pwm_submodule.set_load_mode(flexpwm::LoadMode::reload_full());
        pwm_submodule.set_load_frequency(1);
        pwm_submodule.set_initial_count(&pwm_control, 0);
        pwm_submodule.set_value(ValueRegister::Val0, PwmManager::PWM_FALLING_COUNT);
        pwm_submodule.set_value(ValueRegister::Val1, PwmManager::PWM_RESET_COUNT);
        pwm_submodule.set_interrupts(Interrupts::COMPARE_VAL0 | Interrupts::COMPARE_VAL1);
        pwm_submodule.set_load_ok(&mut pwm_control);
        pwm_submodule.set_running(&mut pwm_control, true);

        let initial_states = [KeyState::Off; NUM_KEYS];

        let mut pwm_manager = PwmManager {
            key_states: initial_states,
            en_al: gpio4.output(pins.p33),
            b1: gpio2.output(pins.p34),
            b2: gpio2.output(pins.p35),
            b3: gpio2.output(pins.p36),
            b4: gpio2.output(pins.p37),
            b5: gpio1.output(pins.p38),
            b6: gpio1.output(pins.p39),
            b7: gpio1.output(pins.p40),
            b8: gpio1.output(pins.p41),
            clk00: gpio2.output(pins.p13),
            clk01: gpio1.output(pins.p14),
            clk02: gpio1.output(pins.p15),
            clk03: gpio1.output(pins.p16),
            clk04: gpio1.output(pins.p17),
            clk05: gpio1.output(pins.p18),
            clk06: gpio1.output(pins.p19),
            clk07: gpio1.output(pins.p20),
            clk08: gpio1.output(pins.p21),
            clk09: gpio1.output(pins.p22),
            clk10: gpio1.output(pins.p23),
        };
        pwm_manager.reset();

        let fsm_manager = FsmManager::new(gpt1, initial_states.clone());

        let mut test_pin = gpio1.output(pins.p0);
        test_pin.set_high().unwrap();

        debug!(logger, "Bothoven ready");

        (
            Shared {
                logger,
                key_states: initial_states,
                modules_to_update: [false; ModuleIndex::NUM_MODULES],
                pwm_manager,
                midi_packets,
            },
            Local {
                fsm_manager,
                midi,
                serial,
                device,
                test_pin,
                pwm_submodule,
            },
        )
    }

    #[idle(shared = [logger, midi_packets], local = [fsm_manager])]
    fn idle(mut ctx: idle::Context) -> ! {
        let idle::LocalResources { fsm_manager, .. } = ctx.local;
        loop {
            ctx.shared.logger.lock(|logger| {
                ctx.shared.midi_packets.lock(|midi_packets| {
                    while let Some(packet) = midi_packets.dequeue() {
                        debug!(logger, "Dequeued packet: {:?}", packet);
                        handle_midi_packet(logger, packet, fsm_manager);
                    }
                });
                fsm_manager.tick(logger);
            });
        }
    }

    #[task(binds = PWM4_1, shared = [&key_states, modules_to_update, pwm_manager], local = [test_pin, pwm_submodule], priority = 2)]
    fn hold_pwm(mut ctx: hold_pwm::Context) {
        let hold_pwm::LocalResources {
            test_pin,
            pwm_submodule,
            ..
        } = ctx.local;
        (ctx.shared.pwm_manager, ctx.shared.modules_to_update).lock(
            |pwm_manager, modules_to_update| {
                if pwm_submodule.status().intersects(Status::COMPARE_VAL0) {
                    pwm_submodule.clear_status(Status::COMPARE_VAL0);
                    pwm_manager.handle_pwm(
                        ctx.shared.key_states,
                        modules_to_update,
                        PwmEdge::Falling,
                    );
                    let _ = test_pin.set_low();
                } else if pwm_submodule.status().intersects(Status::COMPARE_VAL1) {
                    pwm_submodule.clear_status(Status::COMPARE_VAL1);
                    pwm_manager.handle_pwm(
                        ctx.shared.key_states,
                        modules_to_update,
                        PwmEdge::Rising,
                    );
                    let _ = test_pin.set_high();
                }
            },
        )
    }

    #[task(binds = USB_OTG1, shared = [logger, midi_packets], local = [midi, serial, device], priority = 1)]
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
