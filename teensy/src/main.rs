#![no_std]
#![no_main]

use teensy4_bsp::{
    self as bsp,
    hal::{
        ccm::clock_gate,
        usbd::{BusAdapter, EndpointMemory, EndpointState, Speed},
    },
};
use teensy4_panic as _;

use bsp::{
    board,
    hal::{
        flexpwm,
        gpio::Output,
        gpt::{ClockSource, Gpt, Mode, OutputCompareRegister},
        iomuxc::pads::{
            gpio_ad_b0::{GPIO_AD_B0_02, GPIO_AD_B0_03},
            gpio_emc::GPIO_EMC_04,
        },
    },
};
use core::{hint::spin_loop, ops::Index};
use embedded_hal::digital::v2::OutputPin;
use usb_device::{
    class_prelude::*,
    device::{UsbDeviceBuilder, UsbVidPid},
};
use usbd_midi::{
    data::{
        byte::u7::U7,
        midi::{channel::Channel, message::Message, notes::Note},
        usb_midi::midi_packet_reader::MidiPacketBufferReader,
    },
    midi_device::MidiClass,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct KeyPwm(u8);

impl KeyPwm {
    const MAX_PWM: u8 = 64;
    const MIN_PWM: u8 = 16;
}

impl From<U7> for KeyPwm {
    fn from(value: U7) -> Self {
        let velocity: u8 = value.into();
        if velocity == 0 {
            return KeyPwm(0);
        }
        let u7_max: u8 = U7::MAX.into();
        let pwm = ((velocity as u16) * ((Self::MAX_PWM - Self::MIN_PWM) as u16) / (u7_max as u16))
            as u8
            + Self::MIN_PWM;
        return KeyPwm(pwm);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct KeyIndex(u8);

impl KeyIndex {
    const MIN_KEY_INDEX: u8 = 21;
    const MAX_KEY_INDEX: u8 = 108;
}

#[derive(Debug)]
struct InvalidKeyIndex;

impl TryFrom<Note> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: Note) -> Result<Self, Self::Error> {
        let idx: u8 = value.into();
        idx.try_into()
    }
}

impl TryFrom<u8> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if Self::MIN_KEY_INDEX <= value && value <= Self::MAX_KEY_INDEX {
            Ok(KeyIndex(value))
        } else {
            Err(InvalidKeyIndex)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum KeyState {
    Off,
    Pressing { timeout: u32, pwm: KeyPwm },
    Holding { timeout: u32 },
    Repeating { timeout: u32, pwm: KeyPwm },
    Releasing { timeout: u32 },
}

struct PwmManager {
    clk: Output<GPIO_AD_B0_03>,
    data: Output<GPIO_AD_B0_02>,
    latch: Output<GPIO_EMC_04>,
    output_timer: Gpt<1>,
    tick_timer: Gpt<2>,
    last_tick: u32,
    keys: [KeyState; 88],
    _pedal: KeyState,
}

impl PwmManager {
    const PRESS_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    fn delay(&self) {
        self.output_timer.clear_elapsed(OutputCompareRegister::OCR1);
        while !self.output_timer.is_elapsed(OutputCompareRegister::OCR1) {
            spin_loop();
        }
    }
    fn set_key_pwm(&mut self, idx: KeyIndex, pwm: KeyPwm) {
        let tx = ((idx.0 as u16) << 7) + pwm.0 as u16;
        for i in (0..14).rev() {
            let _ = self.data.set_state((tx & (1 << i) != 0).into());
            self.delay();
            let _ = self.clk.set_high();
            self.delay();
            let _ = self.clk.set_low();
        }
        let _ = self.data.set_low();
        let _ = self.latch.set_high();
        self.delay();
        let _ = self.clk.set_high();
        self.delay();
        let _ = self.clk.set_low();
        let _ = self.latch.set_low();
    }
    fn tick(&mut self) {
        let current = self.tick_timer.count();
        self.tick_timer.reset();
        let elapsed = if self.tick_timer.is_rollover() {
            self.tick_timer.clear_rollover();
            (u32::MAX - self.last_tick) + current
        } else {
            current - self.last_tick
        };
        self.last_tick = current;
        for idx in 0..self.keys.len() {
            match self.keys[idx] {
                KeyState::Off => (),
                KeyState::Pressing { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                    0 => self.hold(KeyIndex(idx.try_into().unwrap())),
                    t => self.keys[idx] = KeyState::Pressing { timeout: t, pwm },
                },
                KeyState::Holding { timeout } => match timeout.saturating_sub(elapsed) {
                    0 => self.release(KeyIndex(idx.try_into().unwrap())),
                    t => self.keys[idx] = KeyState::Holding { timeout: t },
                },
                KeyState::Releasing { timeout } => match timeout.saturating_sub(elapsed) {
                    0 => self.off(KeyIndex(idx.try_into().unwrap())),
                    t => self.keys[idx] = KeyState::Releasing { timeout: t },
                },
                KeyState::Repeating { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                    0 => self.press(KeyIndex(idx.try_into().unwrap()), pwm),
                    t => self.keys[idx] = KeyState::Repeating { timeout: t, pwm },
                },
            }
        }
    }
    fn off(&mut self, key: KeyIndex) {
        self.keys[key.0 as usize] = KeyState::Off;
        self.set_key_pwm(key, KeyPwm(0));
    }
    fn press(&mut self, key: KeyIndex, pwm: KeyPwm) {
        self.keys[key.0 as usize] = KeyState::Pressing {
            timeout: Self::PRESS_TIMEOUT_US,
            pwm,
        };
        self.set_key_pwm(key, pwm);
    }
    fn hold(&mut self, key: KeyIndex) {
        self.keys[key.0 as usize] = KeyState::Holding {
            timeout: Self::HOLD_TIMEOUT_US,
        };
        self.set_key_pwm(key, KeyPwm(16));
    }
    fn release(&mut self, key: KeyIndex) {
        self.keys[key.0 as usize] = KeyState::Releasing {
            timeout: Self::RELEASE_TIMEOUT_US,
        };
        self.set_key_pwm(key, KeyPwm(0));
    }
    fn repeat(&mut self, key: KeyIndex, pwm: KeyPwm) {
        self.keys[key.0 as usize] = KeyState::Repeating {
            timeout: Self::REPEAT_TIMEOUT_US,
            pwm,
        };
        self.set_key_pwm(key, KeyPwm(0));
    }
}

impl Index<KeyIndex> for PwmManager {
    type Output = KeyState;
    fn index(&self, index: KeyIndex) -> &Self::Output {
        &self.keys[index.0 as usize]
    }
}

static EP_MEM: EndpointMemory<1024> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();

const CHANNEL: Channel = Channel::Channel1;

#[bsp::rt::entry]
fn main() -> ! {
    let instances = board::instances();
    let board::Resources {
        mut gpio1,
        mut gpio4,
        mut gpt1,
        mut gpt2,
        pins,
        flexpwm4,
        usb,
        mut ccm,
        ..
    } = board::t41(instances);

    let (mut pwm, (_, _, mut sm, _)) = flexpwm4;
    sm.set_debug_enable(true);
    sm.set_wait_enable(true);
    sm.set_clock_select(flexpwm::ClockSelect::Ipg);
    sm.set_prescaler(flexpwm::Prescaler::Prescaler1);
    sm.set_pair_operation(flexpwm::PairOperation::Independent);
    sm.set_load_mode(flexpwm::LoadMode::reload_full());
    sm.set_load_frequency(1);
    sm.set_initial_count(&pwm, 0);
    sm.set_value(flexpwm::FULL_RELOAD_VALUE_REGISTER, 32);
    let clk_cnt = flexpwm::Output::new_b(pins.p3);
    clk_cnt.set_turn_off(&sm, 0);
    clk_cnt.set_turn_on(&sm, 16);
    clk_cnt.set_output_enable(&mut pwm, true);
    sm.set_load_ok(&mut pwm);
    sm.set_running(&mut pwm, true);

    gpt1.set_output_compare_count(OutputCompareRegister::OCR1, 10);
    gpt1.set_clock_source(ClockSource::PeripheralClock);
    gpt1.set_mode(Mode::Restart);
    gpt1.set_divider(1);
    gpt1.enable();

    gpt2.set_clock_source(ClockSource::PeripheralClock);
    gpt2.set_mode(Mode::FreeRunning);
    gpt2.set_divider(1);
    gpt2.enable();

    let mut pwm_manager = PwmManager {
        clk: gpio1.output(pins.p0),
        data: gpio1.output(pins.p1),
        latch: gpio4.output(pins.p2),
        output_timer: gpt1,
        tick_timer: gpt2,
        last_tick: 0,
        keys: [KeyState::Off; 88],
        _pedal: KeyState::Off,
    };

    clock_gate::usb().set(&mut ccm, clock_gate::Setting::On);
    let bus_adapter = BusAdapter::with_speed(usb, &EP_MEM, &EP_STATE, Speed::LowFull);
    let bus_allocator = UsbBusAllocator::new(bus_adapter);
    let mut midi = MidiClass::new(&bus_allocator, 1, 1).unwrap();
    let mut device = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x1234, 0x5678))
        .product("Bothoven")
        .device_class(0x00)
        .device_sub_class(0x00)
        .build();
    loop {
        if device.poll(&mut [&mut midi]) {
            let state = device.state();
            if state == usb_device::device::UsbDeviceState::Configured {
                break;
            }
        }
    }
    device.bus().configure();

    loop {
        pwm_manager.tick();

        if !device.poll(&mut [&mut midi]) {
            continue;
        }

        let mut buffer = [0; 64];

        if let Ok(size) = midi.read(&mut buffer) {
            let buffer_reader = MidiPacketBufferReader::new(&buffer, size);
            for packet in buffer_reader.into_iter() {
                if let Ok(packet) = packet {
                    match packet.message {
                        Message::NoteOn(CHANNEL, note, velocity) => {
                            if let Ok(key) = note.try_into() {
                                match pwm_manager[key] {
                                    KeyState::Off => pwm_manager.press(key, velocity.into()),
                                    KeyState::Holding { .. } | KeyState::Releasing { .. } => {
                                        pwm_manager.repeat(key, velocity.into())
                                    }
                                    KeyState::Pressing { .. } | KeyState::Repeating { .. } => (),
                                }
                            }
                        }
                        Message::NoteOff(CHANNEL, note, _) => {
                            if let Ok(key) = note.try_into() {
                                match pwm_manager[key] {
                                    KeyState::Pressing { .. }
                                    | KeyState::Holding { .. }
                                    | KeyState::Repeating { .. } => pwm_manager.release(key),
                                    KeyState::Off | KeyState::Releasing { .. } => (),
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
