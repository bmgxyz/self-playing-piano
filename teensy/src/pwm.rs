use core::fmt::Write;
use embedded_hal::i2c::I2c;
use midi_convert::midi_types::Value7;
use teensy4_bsp::{
    board::Lpi2c1,
    hal::gpt::{Gpt, Gpt1},
};

use crate::{debug, state::KeyState, warn, KeyIndex, Logger, NUM_KEYS};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct KeyPwm(u8);

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

pub(crate) struct InvalidKeyPwm;

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
            KeyState::Repeating { .. } => KeyPwm::OFF,
            KeyState::Releasing { .. } => KeyPwm::OFF,
        }
    }
}

impl From<KeyPwm> for u8 {
    fn from(value: KeyPwm) -> Self {
        value.0
    }
}

pub(crate) struct PwmManager {
    i2c: Lpi2c1,
    tick_timer: Gpt<1>,
    last_tick: u32,
    pub(crate) key_states: [KeyState; NUM_KEYS],
    needs_update: [bool; NUM_KEYS],
    _pedal: KeyState,
}

impl PwmManager {
    const PRESS_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    pub(crate) fn new(i2c: Lpi2c1, gpt1: Gpt1) -> Self {
        PwmManager {
            i2c,
            key_states: [KeyState::Off; NUM_KEYS],
            needs_update: [false; NUM_KEYS],
            tick_timer: gpt1,
            last_tick: 0,
            _pedal: KeyState::default(),
        }
    }
    pub(crate) fn set_key_state(&mut self, idx: KeyIndex, new_state: KeyState) {
        let current_state = self.key_states[idx];
        self.needs_update[idx] =
            !current_state.same_duty_cycle(&new_state) || self.needs_update[idx];
        self.key_states[idx] = new_state;
    }
    pub(crate) fn off(&mut self, idx: KeyIndex) {
        self.set_key_state(idx, KeyState::Off);
    }
    pub(crate) fn press(&mut self, idx: KeyIndex, pwm: KeyPwm) {
        self.set_key_state(
            idx,
            KeyState::Pressing {
                timeout: Self::PRESS_TIMEOUT_US,
                pwm,
            },
        );
    }
    pub(crate) fn hold(&mut self, idx: KeyIndex) {
        self.set_key_state(
            idx,
            KeyState::Holding {
                timeout: Self::HOLD_TIMEOUT_US,
            },
        );
    }
    pub(crate) fn repeat(&mut self, idx: KeyIndex, pwm: KeyPwm) {
        self.set_key_state(
            idx,
            KeyState::Repeating {
                timeout: Self::REPEAT_TIMEOUT_US,
                pwm,
            },
        );
    }
    pub(crate) fn release(&mut self, idx: KeyIndex) {
        self.set_key_state(
            idx,
            KeyState::Releasing {
                timeout: Self::RELEASE_TIMEOUT_US,
            },
        );
    }
    fn send_update(&mut self, logger: &mut Logger, idx: KeyIndex) {
        let key_state = self.key_states[idx];
        let subcontroller_addr = <KeyIndex as Into<u8>>::into(idx) / 11;
        let local_idx = <KeyIndex as Into<u8>>::into(idx) % 11;
        let pwm: KeyPwm = (key_state).into();
        debug!(logger, "UPDATE {idx:?} {pwm:?}");
        if let Err(e) = self.i2c.write(subcontroller_addr, &[local_idx, pwm.into()]) {
            warn!(logger, "Failed to update key at {idx:?} ({subcontroller_addr}, {local_idx}) due to I2C error: {e:?}",);
        }
    }
    pub(crate) fn tick(&mut self, logger: &mut Logger) {
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
        for key_idx in (0..NUM_KEYS).filter_map(|idx| idx.try_into().ok()) {
            match self.key_states[key_idx] {
                KeyState::Off => (),
                KeyState::Pressing { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                    0 => self.hold(key_idx),
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
                    0 => self.off(key_idx),
                    timeout => self.set_key_state(key_idx, KeyState::Releasing { timeout }),
                },
                KeyState::Repeating { timeout, pwm } => match timeout.saturating_sub(elapsed) {
                    0 => self.press(key_idx, pwm),
                    timeout => self.set_key_state(key_idx, KeyState::Repeating { timeout, pwm }),
                },
            }
            if self.needs_update[key_idx] {
                self.send_update(logger, key_idx);
            }
        }
        self.needs_update = [false; NUM_KEYS];
    }
}
