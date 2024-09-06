#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_spi_Transfer;
use teensy4_bsp::{
    self as bsp,
    board::LpspiPins,
    hal::{
        ccm::{analog::pll2, clock_gate, lpspi_clk},
        iomuxc::pads::gpio_b0::{GPIO_B0_00, GPIO_B0_01, GPIO_B0_02, GPIO_B0_03},
        lpspi::{BitOrder, Lpspi, SamplePoint},
        usbd::{BusAdapter, EndpointMemory, EndpointState, Speed},
    },
};
use teensy4_panic as _;

use bsp::{
    board,
    hal::gpt::{ClockSource, Gpt, Mode},
};
use core::ops::Index;
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
    const OFF: KeyPwm = KeyPwm(0);
    const HOLDING: KeyPwm = KeyPwm(16);

    fn map_velocity_to_pwm(velocity: u8) -> u8 {
        ((velocity as u16) * ((Self::MAX_PWM - Self::MIN_PWM) as u16) / (127u16)) as u8
            + Self::MIN_PWM
    }
}

struct InvalidKeyPwm;

impl TryFrom<u8> for KeyPwm {
    type Error = InvalidKeyPwm;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(KeyPwm::OFF),
            v if v > 127 => Err(InvalidKeyPwm),
            v => {
                let pwm = KeyPwm::map_velocity_to_pwm(v);
                Ok(KeyPwm(pwm))
            }
        }
    }
}

impl From<U7> for KeyPwm {
    fn from(value: U7) -> Self {
        let velocity: u8 = value.into();
        match velocity {
            0 => KeyPwm::OFF,
            v => {
                let pwm = KeyPwm::map_velocity_to_pwm(v);
                KeyPwm(pwm)
            }
        }
    }
}

struct Subcontroller(u8);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct KeyIndex(u8);

impl KeyIndex {
    const MIN_MIDI_PITCH: u8 = 21;
    const NUM_KEYS: u8 = 88;

    fn subcontroller(&self) -> Subcontroller {
        Subcontroller(self.0 / 11)
    }
}

#[derive(Debug)]
struct InvalidKeyIndex;

impl TryFrom<Note> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: Note) -> Result<Self, Self::Error> {
        let idx: u8 = value.into();
        if idx < Self::MIN_MIDI_PITCH {
            return Err(InvalidKeyIndex);
        }
        (idx - Self::MIN_MIDI_PITCH).try_into()
    }
}

impl TryFrom<usize> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let idx: u8 = match value.try_into() {
            Ok(i) => i,
            Err(_) => return Err(InvalidKeyIndex),
        };
        idx.try_into()
    }
}

impl TryFrom<u8> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < Self::NUM_KEYS {
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

type Spi = Lpspi<LpspiPins<GPIO_B0_02, GPIO_B0_01, GPIO_B0_03, GPIO_B0_00>, 4>;

struct PwmManager {
    spi: Spi,
    tick_timer: Gpt<1>,
    last_tick: u32,
    keys: [KeyState; 88],
    _pedal: KeyState,
}

impl PwmManager {
    const PRESS_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    fn set_key_pwm(&mut self, idx: KeyIndex, pwm: KeyPwm) {
        let mut buf = [0u8; 256];
        for (idx, byte) in buf.iter_mut().enumerate() {
            if idx < pwm.0 as usize && idx < 128 {
                *byte = 0xff;
            }
        }
        let _ = match idx.subcontroller() {
            Subcontroller(0) => self.spi.transfer(&mut buf),
            _ => todo!(),
        };
    }
    fn tick(&mut self) {
        let current = self.tick_timer.count();
        let elapsed = if self.tick_timer.is_rollover() {
            self.tick_timer.clear_rollover();
            self.tick_timer.reset();
            u32::MAX
                .saturating_sub(self.last_tick)
                .saturating_add(current)
        } else {
            current.saturating_sub(self.last_tick)
        };
        self.last_tick = current;
        for idx in 0..self.keys.len() {
            if let Ok(key_idx) = idx.try_into() {
                match self[key_idx] {
                    KeyState::Off => (),
                    KeyState::Pressing { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                        0 => self.hold(key_idx),
                        timeout => self.keys[idx] = KeyState::Pressing { timeout, pwm },
                    },
                    KeyState::Holding { timeout } => match timeout.saturating_sub(elapsed) {
                        0 => self.release(key_idx),
                        timeout => self.keys[idx] = KeyState::Holding { timeout },
                    },
                    KeyState::Releasing { timeout } => match timeout.saturating_sub(elapsed) {
                        0 => self.off(key_idx),
                        timeout => self.keys[idx] = KeyState::Releasing { timeout },
                    },
                    KeyState::Repeating { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                        0 => self.press(key_idx, pwm),
                        timeout => self.keys[idx] = KeyState::Repeating { timeout, pwm },
                    },
                }
            }
        }
    }
    fn off(&mut self, key: KeyIndex) {
        self.keys[key.0 as usize] = KeyState::Off;
        self.set_key_pwm(key, KeyPwm::OFF);
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
        self.set_key_pwm(key, KeyPwm::HOLDING);
    }
    fn release(&mut self, key: KeyIndex) {
        self.keys[key.0 as usize] = KeyState::Releasing {
            timeout: Self::RELEASE_TIMEOUT_US,
        };
        self.set_key_pwm(key, KeyPwm::OFF);
    }
    fn repeat(&mut self, key: KeyIndex, pwm: KeyPwm) {
        self.keys[key.0 as usize] = KeyState::Repeating {
            timeout: Self::REPEAT_TIMEOUT_US,
            pwm,
        };
        self.set_key_pwm(key, KeyPwm::OFF);
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

const LPSPI_CLK_DIVIDER: u32 = 4;
const LPSPI_CLK_HZ: u32 = pll2::FREQUENCY / LPSPI_CLK_DIVIDER;
const SCK_HZ: u32 = 512_000;

#[bsp::rt::entry]
fn main() -> ! {
    let instances = board::instances();
    let board::Resources {
        mut ccm,
        mut gpt1,
        lpspi4,
        pins,
        usb,
        ..
    } = board::t41(instances);

    clock_gate::lpspi::<2>().set(&mut ccm, clock_gate::OFF);
    lpspi_clk::set_selection(&mut ccm, lpspi_clk::Selection::Pll2);
    lpspi_clk::set_divider(&mut ccm, LPSPI_CLK_DIVIDER);
    clock_gate::lpspi::<2>().set(&mut ccm, clock_gate::ON);
    let mut spi = board::lpspi(
        lpspi4,
        board::LpspiPins {
            sdo: pins.p11,
            sdi: pins.p12,
            sck: pins.p13,
            pcs0: pins.p10,
        },
        SCK_HZ,
    );
    spi.set_bit_order(BitOrder::Msb);
    spi.disabled(|spi| {
        spi.set_clock_hz(LPSPI_CLK_HZ, SCK_HZ);
        spi.set_sample_point(SamplePoint::Edge);
    });

    gpt1.set_clock_source(ClockSource::PeripheralClock);
    gpt1.set_mode(Mode::FreeRunning);
    gpt1.set_divider(1);
    gpt1.enable();

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

    let mut pwm_manager = PwmManager {
        spi,
        tick_timer: gpt1,
        last_tick: 0,
        keys: [KeyState::Off; 88],
        _pedal: KeyState::Off,
    };

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
