#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_spi_Transfer;
use embedded_hal::digital::v2::{OutputPin, PinState};
use teensy4_bsp::{
    self as bsp,
    board::LpspiPins,
    hal::{
        ccm::{analog::pll2, clock_gate, lpspi_clk},
        gpio::Output,
        iomuxc::pads::{
            gpio_ad_b0::{GPIO_AD_B0_02, GPIO_AD_B0_03},
            gpio_b0::{GPIO_B0_00, GPIO_B0_01, GPIO_B0_02, GPIO_B0_03, GPIO_B0_10},
            gpio_b1::GPIO_B1_01,
            gpio_emc::{GPIO_EMC_04, GPIO_EMC_05, GPIO_EMC_06, GPIO_EMC_08},
        },
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

#[derive(Debug, Clone, Copy)]
struct Subcontroller {
    needs_update: bool,
    keys: [KeyState; 11],
}

impl Default for Subcontroller {
    fn default() -> Self {
        Subcontroller {
            needs_update: false,
            keys: [KeyState::default(); 11],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct KeyIndex(u8);

impl KeyIndex {
    const MIN_MIDI_PITCH: u8 = 21;
    const NUM_KEYS: u8 = 88;

    fn get_subcontroller_idxs(&self) -> (usize, usize) {
        (self.0 as usize / 11, self.0 as usize % 11)
    }
}

impl Index<KeyIndex> for Subcontroller {
    type Output = KeyState;
    fn index(&self, index: KeyIndex) -> &Self::Output {
        &self.keys[index.0 as usize]
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

impl KeyState {
    fn same_state(&self, other: &KeyState) -> bool {
        match (self, other) {
            (KeyState::Off, KeyState::Off)
            | (KeyState::Pressing { .. }, KeyState::Pressing { .. })
            | (KeyState::Holding { .. }, KeyState::Holding { .. })
            | (KeyState::Repeating { .. }, KeyState::Repeating { .. })
            | (KeyState::Releasing { .. }, KeyState::Releasing { .. }) => true,
            _ => false,
        }
    }
}

impl Default for KeyState {
    fn default() -> Self {
        KeyState::Off
    }
}

type Spi = Lpspi<LpspiPins<GPIO_B0_02, GPIO_B0_01, GPIO_B0_03, GPIO_B0_00>, 4>;

struct PwmManager {
    spi: Spi,
    cs0: Output<GPIO_AD_B0_03>,
    cs1: Output<GPIO_AD_B0_02>,
    cs2: Output<GPIO_EMC_04>,
    cs3: Output<GPIO_EMC_05>,
    cs4: Output<GPIO_EMC_06>,
    cs5: Output<GPIO_EMC_08>,
    cs6: Output<GPIO_B0_10>,
    cs7: Output<GPIO_B1_01>,
    tick_timer: Gpt<1>,
    last_tick: u32,
    subcontrollers: [Subcontroller; 8],
    _pedal: KeyState,
}

impl PwmManager {
    const PRESS_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    fn reset(&mut self) {
        for idx in 0..self.subcontrollers.len() {
            self.set_cs_state(idx, PinState::High);
        }
    }
    fn set_cs_state(&mut self, idx: usize, state: PinState) {
        let _ = match idx {
            0 => self.cs0.set_state(state),
            1 => self.cs1.set_state(state),
            2 => self.cs2.set_state(state),
            3 => self.cs3.set_state(state),
            4 => self.cs4.set_state(state),
            5 => self.cs5.set_state(state),
            6 => self.cs6.set_state(state),
            7 => self.cs7.set_state(state),
            _ => Ok(()),
        };
    }
    fn update_subcontroller(&mut self, idx: usize) {
        self.set_cs_state(idx, PinState::Low);
        let mut new_pwm_bytes = [0u8; 256];
        for pwm_idx in 0..128 {
            for key_idx in 0..6 {
                match self.get_key_state(KeyIndex(key_idx)) {
                    KeyState::Pressing { pwm, .. } => {
                        if pwm.0 > pwm_idx {
                            new_pwm_bytes[pwm_idx as usize] |= 1 << key_idx
                        }
                    }
                    KeyState::Holding { .. } => {
                        if KeyPwm::HOLDING.0 > pwm_idx {
                            new_pwm_bytes[pwm_idx as usize] |= 1 << key_idx
                        }
                    }
                    KeyState::Off | KeyState::Releasing { .. } | KeyState::Repeating { .. } => (),
                }
            }
            for key_idx in 6..11 {
                match self.get_key_state(KeyIndex(key_idx)) {
                    KeyState::Pressing { pwm, .. } => {
                        if pwm.0 > pwm_idx {
                            new_pwm_bytes[pwm_idx as usize + 128] |= 1 << key_idx
                        }
                    }
                    KeyState::Holding { .. } => {
                        if KeyPwm::HOLDING.0 > pwm_idx {
                            new_pwm_bytes[pwm_idx as usize + 128] |= 1 << key_idx
                        }
                    }
                    KeyState::Off | KeyState::Releasing { .. } | KeyState::Repeating { .. } => (),
                }
            }
        }
        let _ = self.spi.transfer(&mut new_pwm_bytes);
        self.set_cs_state(idx, PinState::High);
        self.subcontrollers[idx].needs_update = false;
    }
    fn set_key_state(&mut self, idx: KeyIndex, state: KeyState) {
        let (subcontroller_idx, key_idx) = idx.get_subcontroller_idxs();
        let subcontroller = &mut self.subcontrollers[subcontroller_idx];
        let current_state = &mut subcontroller.keys[key_idx];
        if !current_state.same_state(&state) {
            subcontroller.needs_update = true;
        }
        *current_state = state;
    }
    fn get_key_state(&self, idx: KeyIndex) -> KeyState {
        let (subcontroller_idx, key_idx) = idx.get_subcontroller_idxs();
        self.subcontrollers[subcontroller_idx].keys[key_idx]
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
        for idx in 0u8..=88 {
            if let Ok(key_idx) = idx.try_into() {
                match self.get_key_state(key_idx) {
                    KeyState::Off => (),
                    KeyState::Pressing { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                        0 => self.set_key_state(
                            key_idx,
                            KeyState::Holding {
                                timeout: Self::HOLD_TIMEOUT_US,
                            },
                        ),
                        timeout => self.set_key_state(key_idx, KeyState::Pressing { timeout, pwm }),
                    },
                    KeyState::Holding { timeout } => match timeout.saturating_sub(elapsed) {
                        0 => self.set_key_state(
                            key_idx,
                            KeyState::Releasing {
                                timeout: Self::RELEASE_TIMEOUT_US,
                            },
                        ),
                        timeout => self.set_key_state(key_idx, KeyState::Holding { timeout }),
                    },
                    KeyState::Releasing { timeout } => match timeout.saturating_sub(elapsed) {
                        0 => self.set_key_state(key_idx, KeyState::Off),
                        timeout => self.set_key_state(key_idx, KeyState::Releasing { timeout }),
                    },
                    KeyState::Repeating { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                        0 => self.set_key_state(
                            key_idx,
                            KeyState::Pressing {
                                timeout: Self::PRESS_TIMEOUT_US,
                                pwm,
                            },
                        ),
                        timeout => {
                            self.set_key_state(key_idx, KeyState::Repeating { timeout, pwm })
                        }
                    },
                }
            }
        }
        for idx in 0..self.subcontrollers.len() {
            if self.subcontrollers[idx].needs_update {
                self.update_subcontroller(idx);
            }
        }
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
        mut gpio1,
        mut gpio2,
        mut gpio4,
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

    let subcontrollers = [Subcontroller::default(); 8];
    let mut pwm_manager = PwmManager {
        spi,
        cs0: gpio1.output(pins.p0),
        cs1: gpio1.output(pins.p1),
        cs2: gpio4.output(pins.p2),
        cs3: gpio4.output(pins.p3),
        cs4: gpio4.output(pins.p4),
        cs5: gpio4.output(pins.p5),
        cs6: gpio2.output(pins.p6),
        cs7: gpio2.output(pins.p7),
        tick_timer: gpt1,
        last_tick: 0,
        subcontrollers,
        _pedal: KeyState::default(),
    };
    pwm_manager.reset();

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
                            if let Ok(key_idx) = note.try_into() {
                                match pwm_manager.get_key_state(key_idx) {
                                    KeyState::Off => pwm_manager.set_key_state(
                                        key_idx,
                                        KeyState::Pressing {
                                            timeout: PwmManager::PRESS_TIMEOUT_US,
                                            pwm: velocity.into(),
                                        },
                                    ),
                                    KeyState::Holding { .. } | KeyState::Releasing { .. } => {
                                        pwm_manager.set_key_state(
                                            key_idx,
                                            KeyState::Repeating {
                                                timeout: PwmManager::REPEAT_TIMEOUT_US,
                                                pwm: velocity.into(),
                                            },
                                        )
                                    }
                                    KeyState::Pressing { .. } | KeyState::Repeating { .. } => (),
                                }
                            }
                        }
                        Message::NoteOff(CHANNEL, note, _) => {
                            if let Ok(key_idx) = note.try_into() {
                                match pwm_manager.get_key_state(key_idx) {
                                    KeyState::Pressing { .. }
                                    | KeyState::Holding { .. }
                                    | KeyState::Repeating { .. } => pwm_manager.set_key_state(
                                        key_idx,
                                        KeyState::Releasing {
                                            timeout: PwmManager::RELEASE_TIMEOUT_US,
                                        },
                                    ),
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
