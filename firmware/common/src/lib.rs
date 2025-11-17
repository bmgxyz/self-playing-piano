#![no_std]

use core::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use bounded_integer::bounded_integer;
use heapless::Vec;
use serde::{Deserialize, Serialize};

pub const NUM_KEYS: usize = 88;
const NUM_PINS_PER_MODULE: usize = 12;

pub const SERIAL_BUF_SIZE: usize = 128;
pub const SERIAL_BAUD_RATE: u32 = 115_200;

bounded_integer! { pub struct KeyIndex(0, 87); }
bounded_integer! { pub struct ModuleIndex(0, 7); }
bounded_integer! { pub struct ModuleKeyIndex(0, 11); }
bounded_integer! { pub struct KeyVelocity(0, 127); }
bounded_integer! { pub struct DutyCyclePercent(0, 100); }

impl KeyIndex {
    pub fn get_module_index(&self) -> ModuleIndex {
        ModuleIndex::new((self.get() + 4) / NUM_PINS_PER_MODULE as u8).unwrap()
    }
    pub fn get_module_key_index(&self) -> ModuleKeyIndex {
        ModuleKeyIndex::new((self.get() + 4) % NUM_PINS_PER_MODULE as u8).unwrap()
    }
    pub fn from_module_indices(
        module_index: &ModuleIndex,
        module_key_index: &ModuleKeyIndex,
    ) -> Option<KeyIndex> {
        KeyIndex::new(
            (module_index.get() * NUM_PINS_PER_MODULE as u8 + module_key_index.get())
                .saturating_sub(4),
        )
    }
}

#[test]
fn key_index() {
    for key_index_num in KeyIndex::MIN_VALUE..KeyIndex::MAX_VALUE {
        let key_index = KeyIndex::new(key_index_num).unwrap();
        assert_eq!(
            key_index,
            KeyIndex::from_module_indices(
                &key_index.get_module_index(),
                &key_index.get_module_key_index()
            )
            .unwrap()
        );
    }
}

impl ModuleIndex {
    pub fn get_first_key_index(&self) -> KeyIndex {
        KeyIndex::from_module_indices(self, &ModuleKeyIndex::const_new::<0>()).unwrap()
    }
}

impl Index<ModuleKeyIndex> for ModuleKeyStates {
    type Output = KeyState;

    fn index(&self, index: ModuleKeyIndex) -> &Self::Output {
        &self[index.get() as usize]
    }
}

impl IndexMut<ModuleKeyIndex> for ModuleKeyStates {
    fn index_mut(&mut self, index: ModuleKeyIndex) -> &mut Self::Output {
        &mut self[index.get() as usize]
    }
}

impl Index<KeyIndex> for KeyStates {
    type Output = KeyState;

    fn index(&self, index: KeyIndex) -> &Self::Output {
        &self[index.get() as usize]
    }
}

impl IndexMut<KeyIndex> for KeyStates {
    fn index_mut(&mut self, index: KeyIndex) -> &mut Self::Output {
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
    const TOTAL_PWM_PERIOD_US: u8 = 50;

    pub const fn empty() -> Schedule {
        Schedule {
            actions: Vec::new(),
        }
    }
    pub fn all_off() -> Schedule {
        Self {
            actions: Vec::from_slice(&[
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<0>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<1>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<2>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<3>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<4>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<5>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<6>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<7>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<8>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<9>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<10>(),
                    new_state: false,
                },
                Action::Transition {
                    module_key_index: ModuleKeyIndex::const_new::<11>(),
                    new_state: false,
                },
                Action::Delay {
                    duration_us: Self::TOTAL_PWM_PERIOD_US,
                },
            ])
            .unwrap(),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8, SERIAL_BUF_SIZE>, postcard::Error> {
        postcard::to_vec(self)
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Schedule, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    pub fn build(module_index: &ModuleIndex, key_states: &KeyStates) -> Schedule {
        let states: Vec<(DutyCyclePercent, ModuleKeyIndex), NUM_PINS_PER_MODULE> = key_states
            .iter()
            .skip(module_index.get_first_key_index().into())
            .take(NUM_PINS_PER_MODULE)
            .enumerate()
            .map(|(idx, ks)| {
                (
                    ks.get_duty_cycle(),
                    ModuleKeyIndex::new_saturating(idx as u8),
                )
            })
            .collect();
        let mut falling_edges: Vec<(DutyCyclePercent, ModuleKeyIndex), NUM_PINS_PER_MODULE> =
            states
                .clone()
                .into_iter()
                .filter(|(dc, _idx)| dc.get() != 0)
                .collect();
        falling_edges.sort_unstable();

        let starting_pin_states = states.iter().map(|(dc, idx)| Action::Transition {
            module_key_index: *idx,
            new_state: *dc > 0,
        });
        let mut schedule = Schedule::empty();
        schedule.actions.extend(starting_pin_states);
        let mut period_position_us = 0;
        for (duty_cycle, module_key_index) in falling_edges.iter() {
            let new_period_position_us =
                (Self::TOTAL_PWM_PERIOD_US as f32 * (duty_cycle.get() as f32) / 100.) as u8;
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
        let delay_duration_us = Self::TOTAL_PWM_PERIOD_US.saturating_sub(period_position_us);
        if delay_duration_us > 0 {
            let _ = schedule.actions.push(Action::Delay {
                duration_us: delay_duration_us,
            });
        }

        schedule
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

pub type KeyStates = [KeyState; NUM_KEYS];
pub type ModuleKeyStates = [KeyState; NUM_PINS_PER_MODULE];

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

    pub fn midi_note_on(&mut self, velocity: &KeyVelocity) {
        match self {
            KeyState::Off | KeyState::Pressing { .. } => self.press(*velocity),
            KeyState::Holding { .. } | KeyState::Repeating { .. } | KeyState::Releasing { .. } => {
                self.repeat(*velocity)
            }
        }
    }
    pub fn midi_note_off(&mut self) {
        match self {
            KeyState::Off => self.off(),
            KeyState::Pressing { .. }
            | KeyState::Holding { .. }
            | KeyState::Repeating { .. }
            | KeyState::Releasing { .. } => self.release(),
        }
    }

    fn off(&mut self) {
        *self = KeyState::Off;
    }
    fn press(&mut self, velocity: KeyVelocity) {
        *self = KeyState::Pressing {
            timeout_us: Self::PRESSING_TIMEOUT_US,
            velocity,
        };
    }
    fn hold(&mut self) {
        *self = KeyState::Holding {
            timeout_us: Self::HOLD_TIMEOUT_US,
        };
    }
    fn repeat(&mut self, velocity: KeyVelocity) {
        *self = KeyState::Repeating {
            timeout_us: Self::REPEAT_TIMEOUT_US,
            velocity,
        };
    }
    fn release(&mut self) {
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
                let velocity = *velocity;
                match timeout_us.saturating_sub(elapsed_us) {
                    0 => self.press(velocity),
                    timeout_us => {
                        *self = KeyState::Repeating {
                            timeout_us,
                            velocity,
                        }
                    }
                }
            }
        }
    }
}
