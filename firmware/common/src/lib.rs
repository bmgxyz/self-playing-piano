#![no_std]

use core::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use bounded_integer::bounded_integer;
use heapless::Vec;
use serde::{Deserialize, Serialize};

pub const NUM_KEYS: usize = 88;
pub const NUM_KEYS_PER_MODULE: usize = 11;
pub const NUM_MODULES: usize = NUM_KEYS / NUM_KEYS_PER_MODULE;

pub const SERIAL_BUF_SIZE: usize = 128;
pub const SERIAL_BAUD_RATE: u32 = 115_200;

bounded_integer! { pub struct KeyIndex(0, 87); }
bounded_integer! { pub struct ModuleIndex(0, 7); }
bounded_integer! { pub struct ModuleKeyIndex(0, 10); }
bounded_integer! { pub struct KeyVelocity(0, 127); }
bounded_integer! { pub struct DutyCyclePercent(0, 100); }

impl KeyIndex {
    pub fn get_module_index(&self) -> ModuleIndex {
        ModuleIndex::new(self.get() / NUM_KEYS_PER_MODULE as u8).unwrap()
    }
    pub fn get_module_key_index(&self) -> ModuleKeyIndex {
        ModuleKeyIndex::new(self.get() % NUM_KEYS_PER_MODULE as u8).unwrap()
    }
    pub fn from_module_indices(
        module_index: &ModuleIndex,
        module_key_index: &ModuleKeyIndex,
    ) -> Option<KeyIndex> {
        KeyIndex::new(module_index.get() * NUM_KEYS_PER_MODULE as u8 + module_key_index.get())
    }
}

impl Index<ModuleKeyIndex> for [KeyState; NUM_KEYS_PER_MODULE] {
    type Output = KeyState;

    fn index(&self, index: ModuleKeyIndex) -> &Self::Output {
        &self[index.get() as usize]
    }
}

impl IndexMut<ModuleKeyIndex> for [KeyState; NUM_KEYS_PER_MODULE] {
    fn index_mut(&mut self, index: ModuleKeyIndex) -> &mut Self::Output {
        &mut self[index.get() as usize]
    }
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Action {
    Delay {
        duration_us: u8,
    },
    Transition {
        module_key_index: ModuleKeyIndex,
        new_state: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Schedule {
    pub actions: Vec<Action, 48>,
}

impl Schedule {
    pub fn new() -> Schedule {
        Schedule {
            actions: Vec::new(),
        }
    }
    pub fn serialize(&self) -> Result<Vec<u8, SERIAL_BUF_SIZE>, postcard::Error> {
        postcard::to_vec(self)
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Schedule, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for action in self.actions.iter() {
            match action {
                Action::Delay { duration_us } => writeln!(f, "delay for {duration_us} us")?,
                Action::Transition {
                    module_key_index,
                    new_state,
                } => writeln!(
                    f,
                    "set {module_key_index} to {}",
                    match new_state {
                        true => "high",
                        false => "low",
                    }
                )?,
            };
        }
        Ok(())
    }
}

pub const ACK_RESPONSE: [u8; 3] = *b"/\r\n";
pub const NAK_RESPONSE: [u8; 3] = *b"?\r\n";

pub type KeyStates = [KeyState; NUM_KEYS_PER_MODULE];

pub fn build_schedule(key_states: &KeyStates) -> Schedule {
    let mut falling_edges: Vec<(DutyCyclePercent, ModuleKeyIndex), NUM_KEYS_PER_MODULE> =
        key_states
            .iter()
            .enumerate()
            .map(|(idx, ks)| {
                (
                    ks.get_duty_cycle(),
                    ModuleKeyIndex::new_saturating(idx as u8),
                )
            })
            .filter(|(dc, _idx)| dc.get() != 0)
            .collect();

    let mut schedule = Schedule::new();
    if falling_edges.is_empty() {
        return schedule;
    }
    falling_edges.sort_unstable();
    let rising_edges = falling_edges.iter().map(|(_dc, idx)| Action::Transition {
        module_key_index: *idx,
        new_state: true,
    });
    schedule.actions.extend(rising_edges.clone());
    // TODO factor out
    const TOTAL_PWM_PERIOD_US: u8 = 50;
    let mut period_position_us = 0;
    for (duty_cycle, module_key_index) in falling_edges.iter() {
        let new_period_position_us =
            (TOTAL_PWM_PERIOD_US as f32 * (duty_cycle.get() as f32) / 100.) as u8;
        let delay_duration_us = new_period_position_us.saturating_sub(period_position_us);
        if delay_duration_us > 0 {
            let _ = schedule.actions.push(Action::Delay {
                duration_us: delay_duration_us,
            });
        }
        let _ = schedule.actions.push(Action::Transition {
            module_key_index: *module_key_index,
            new_state: false,
        });
        period_position_us = new_period_position_us;
    }
    // handle last delay
    let delay_duration_us = TOTAL_PWM_PERIOD_US.saturating_sub(period_position_us);
    if delay_duration_us > 0 {
        let _ = schedule.actions.push(Action::Delay {
            duration_us: delay_duration_us,
        });
    }

    schedule
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub idx: KeyIndex,
    pub state: KeyState,
}

impl Message {
    pub fn serialize(&self) -> Result<Vec<u8, 32>, postcard::Error> {
        postcard::to_vec(self)
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Message, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum KeyState {
    #[default]
    Off,
    Pressing {
        timeout_us: u32,
        velocity: KeyVelocity,
    },
    Holding {
        timeout_us: u32,
    },
    Repeating {
        timeout_us: u32,
        velocity: KeyVelocity,
    },
    Releasing {
        timeout_us: u32,
    },
}

impl KeyState {
    const PRESSING_TIMEOUT_US: u32 = 100_000;
    const HOLD_TIMEOUT_US: u32 = 30_000_000;
    const RELEASE_TIMEOUT_US: u32 = 100_000;
    const REPEAT_TIMEOUT_US: u32 = Self::RELEASE_TIMEOUT_US;

    const OFF_DUTY_CYCLE_PERCENT: DutyCyclePercent = DutyCyclePercent::const_new::<0>();
    const HOLDING_DUTY_CYCLE_PERCENT: DutyCyclePercent = DutyCyclePercent::const_new::<18>();

    pub fn get_duty_cycle(&self) -> DutyCyclePercent {
        match self {
            KeyState::Off | KeyState::Repeating { .. } | KeyState::Releasing { .. } => {
                Self::OFF_DUTY_CYCLE_PERCENT
            }
            KeyState::Pressing { velocity, .. } => DutyCyclePercent::new_saturating(
                (velocity.get() as f32 / KeyVelocity::MAX_VALUE as f32 * 100.) as u8,
            ),
            KeyState::Holding { .. } => Self::HOLDING_DUTY_CYCLE_PERCENT,
        }
    }

    pub fn off(&mut self) {
        *self = KeyState::Off;
    }
    pub fn press(&mut self, velocity: KeyVelocity) {
        *self = KeyState::Pressing {
            timeout_us: Self::PRESSING_TIMEOUT_US,
            velocity,
        };
    }
    pub fn hold(&mut self) {
        *self = KeyState::Holding {
            timeout_us: Self::HOLD_TIMEOUT_US,
        };
    }
    pub fn repeat(&mut self, velocity: KeyVelocity) {
        *self = KeyState::Repeating {
            timeout_us: Self::REPEAT_TIMEOUT_US,
            velocity,
        };
    }
    pub fn release(&mut self) {
        *self = KeyState::Releasing {
            timeout_us: Self::RELEASE_TIMEOUT_US,
        };
    }

    pub fn tick(&mut self, elapsed_us: u32) {
        match self {
            KeyState::Off => (),
            KeyState::Pressing {
                timeout_us,
                velocity,
            } => match timeout_us.saturating_sub(elapsed_us) {
                0 => self.hold(),
                timeout_us => {
                    *self = KeyState::Pressing {
                        timeout_us,
                        velocity: *velocity,
                    }
                }
            },
            KeyState::Holding { timeout_us } => match timeout_us.saturating_sub(elapsed_us) {
                0 => self.release(),
                timeout_us => *self = KeyState::Holding { timeout_us },
            },
            KeyState::Releasing { timeout_us } => match timeout_us.saturating_sub(elapsed_us) {
                0 => self.off(),
                timeout_us => *self = KeyState::Releasing { timeout_us },
            },
            KeyState::Repeating {
                timeout_us,
                velocity,
            } => {
                let v = velocity.clone();
                match timeout_us.saturating_sub(elapsed_us) {
                    0 => self.press(v),
                    timeout_us => {
                        *self = KeyState::Repeating {
                            timeout_us,
                            velocity: v,
                        }
                    }
                }
            }
        }
    }
}
