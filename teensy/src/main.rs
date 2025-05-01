#![no_std]
#![no_main]

use embedded_hal::i2c::I2c;
use heapless::Vec;
use midi_convert::{
    midi_types::{Channel, MidiMessage, Note, Value7},
    parse::MidiTryParseSlice,
};
use teensy4_bsp::{
    self as bsp,
    board::Lpi2c1,
    hal::{
        ccm::{clock_gate, lpi2c_clk},
        usbd::{BusAdapter, EndpointMemory, EndpointState, Speed},
    },
};
use teensy4_panic as _;

use bsp::{
    board,
    hal::gpt::{ClockSource, Gpt, Mode},
};
use core::{
    fmt::{Error, Write},
    ops::{Index, IndexMut},
    slice::{Iter, IterMut},
};
use usb_device::{
    class_prelude::*,
    device::{StringDescriptors, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_midi::{UsbMidiClass, UsbMidiPacketReader};
use usbd_serial::SerialPort;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct KeyPwm(u8);

impl KeyPwm {
    const MAX_PWM: u8 = 72;
    const MIN_PWM: u8 = 26;
    const OFF: KeyPwm = KeyPwm(0);
    const HOLDING: KeyPwm = KeyPwm(18);

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

impl From<Value7> for KeyPwm {
    fn from(value: Value7) -> Self {
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

impl From<KeyState> for KeyPwm {
    fn from(value: KeyState) -> Self {
        match value {
            KeyState::Off => KeyPwm::OFF,
            KeyState::Pressing { pwm, .. } => pwm,
            KeyState::Holding { .. } => KeyPwm::HOLDING,
            KeyState::Repeating { pwm, .. } => pwm,
            KeyState::Releasing { .. } => KeyPwm::OFF,
        }
    }
}

impl From<KeyPwm> for u8 {
    fn from(value: KeyPwm) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct KeyIndex(u8);

impl KeyIndex {
    const MIN_MIDI_PITCH: u8 = 21;
    const NUM_KEYS: u8 = 88;
}

impl From<KeyIndex> for usize {
    fn from(value: KeyIndex) -> Self {
        value.0.into()
    }
}

impl From<KeyIndex> for u8 {
    fn from(value: KeyIndex) -> Self {
        value.0.into()
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

struct AllKeys([KeyState; 88]);

impl Default for AllKeys {
    fn default() -> Self {
        AllKeys([KeyState::default(); 88])
    }
}

impl Index<KeyIndex> for AllKeys {
    type Output = KeyState;
    fn index(&self, index: KeyIndex) -> &Self::Output {
        let idx: usize = index.into();
        &self.0[idx]
    }
}

impl IndexMut<KeyIndex> for AllKeys {
    fn index_mut(&mut self, index: KeyIndex) -> &mut Self::Output {
        let idx: usize = index.into();
        &mut self.0[idx]
    }
}

impl AllKeys {
    fn iter(&self) -> Iter<'_, KeyState> {
        self.0.iter()
    }
    fn iter_mut(&mut self) -> IterMut<'_, KeyState> {
        self.0.iter_mut()
    }
}

struct PwmManager {
    i2c: Lpi2c1,
    tick_timer: Gpt<1>,
    last_tick: u32,
    current_key_states: AllKeys,
    next_key_states: AllKeys,
    _pedal: KeyState,
}

impl PwmManager {
    const PRESS_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    fn set_key_state(&mut self, idx: KeyIndex, state: KeyState) {
        self.next_key_states[idx] = state;
    }
    fn get_key_state(&self, idx: KeyIndex) -> KeyState {
        self.current_key_states[idx]
    }
    fn tick(&mut self, logger: &mut Logger) {
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
        for (idx, (curr, next)) in self
            .current_key_states
            .iter_mut()
            .zip(self.next_key_states.iter())
            .enumerate()
        {
            if !curr.same_state(next) {
                let key_idx = match idx.try_into() {
                    Ok(i) => <KeyIndex as Into<u8>>::into(i),
                    Err(_) => {
                        warn!(logger, "Failed to update key at index {}", idx);
                        continue;
                    }
                };
                let subcontroller_addr = (key_idx / 11) << 1;
                let local_idx = key_idx % 11;
                let pwm: KeyPwm = (*next).into();
                if let Err(e) = self.i2c.write(subcontroller_addr, &[local_idx, pwm.into()]) {
                    warn!(logger, "Failed to update key at index {idx} ({subcontroller_addr}, {local_idx}) due to I2C error: {e:?}",);
                }
                *curr = *next;
            }
        }
    }
}

static EP_MEM: EndpointMemory<1024> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();

const CHANNEL: Channel = Channel::C1;

struct Logger {
    logs_to_write: Vec<u8, 1024>,
}

impl Logger {
    fn new() -> Self {
        Logger {
            logs_to_write: Vec::new(),
        }
    }
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.logs_to_write.extend_from_slice(s.as_bytes()) {
            Ok(()) => core::fmt::Result::Ok(()),
            Err(_) => core::fmt::Result::Err(Error),
        }
    }
}

#[macro_export]
macro_rules! trace {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = write!($logger, "TRACE: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = write!($logger, "DEBUG: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = write!($logger, "INFO: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warn {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = write!($logger, "WARN: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! error {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = write!($logger, "ERROR: {}", format_args!($($arg)*));
    }};
}

#[bsp::rt::entry]
fn main() -> ! {
    let mut logger = Logger::new();
    let instances = board::instances();
    let board::Resources {
        mut ccm,
        mut gpt1,
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

    gpt1.set_clock_source(ClockSource::PeripheralClock);
    gpt1.set_mode(Mode::FreeRunning);
    gpt1.set_divider(1);
    gpt1.enable();

    clock_gate::usb().set(&mut ccm, clock_gate::Setting::On);
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

    let mut pwm_manager = PwmManager {
        i2c,
        current_key_states: AllKeys::default(),
        next_key_states: AllKeys::default(),
        tick_timer: gpt1,
        last_tick: 0,
        _pedal: KeyState::default(),
    };

    debug!(logger, "Bothoven ready");

    loop {
        if let Ok(len) = serial.write(&logger.logs_to_write) {
            logger.logs_to_write.rotate_left(len);
            logger.logs_to_write.truncate(len);
        }
        pwm_manager.tick(&mut logger);

        if !device.poll(&mut [&mut midi, &mut serial]) {
            continue;
        }

        let mut buffer = [0; 64];

        if let Ok(size) = midi.read(&mut buffer) {
            let buffer_reader = UsbMidiPacketReader::new(&buffer, size);
            for packet in buffer_reader.into_iter() {
                if let Ok(packet) = packet {
                    let message = match MidiMessage::try_parse_slice(packet.payload_bytes()) {
                        Ok(m) => m,
                        Err(e) => {
                            warn!(logger, "Failed to parse MIDI packet {packet:?}: {e:?}");
                            continue;
                        }
                    };
                    match message {
                        MidiMessage::NoteOn(channel, note, velocity) => {
                            if channel != CHANNEL {
                                continue;
                            }
                            debug!(
                                logger,
                                "On {note:?} ({}), {velocity:?}",
                                <Note as Into<u8>>::into(note)
                            );
                            if let Ok(key_idx) = note.try_into() {
                                let key_state = pwm_manager.get_key_state(key_idx);
                                debug!(logger, "{key_idx:?} in {key_state:?}");
                                match key_state {
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
                        MidiMessage::NoteOff(channel, note, _) => {
                            if channel != CHANNEL {
                                continue;
                            }
                            debug!(logger, "Off {note:?} ({})", <Note as Into<u8>>::into(note));
                            if let Ok(key_idx) = note.try_into() {
                                let key_state = pwm_manager.get_key_state(key_idx);
                                debug!(logger, "{key_idx:?} in {key_state:?}");
                                match key_state {
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
