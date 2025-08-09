use fugit::MicrosDurationU32;
use teensy4_bsp::hal::gpt::{ClockSource, Gpt1, Mode};

use crate::{index::KeyIndex, log::Logger, state::KeyState, NUM_KEYS};

pub(crate) struct FsmManager {
    tick_timer: Gpt1,
    last_tick: u32,
    pub(crate) key_states: [KeyState; NUM_KEYS],
    _pedal: KeyState,
}

type KeyPressDuration = MicrosDurationU32;

impl FsmManager {
    const HOLD_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(30_000_000);
    const RELEASE_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
    const REPEAT_TIMEOUT: MicrosDurationU32 = Self::RELEASE_TIMEOUT;

    pub(crate) fn new(mut gpt1: Gpt1, initial_state: [KeyState; NUM_KEYS]) -> Self {
        gpt1.set_clock_source(ClockSource::PeripheralClock);
        gpt1.set_mode(Mode::FreeRunning);
        gpt1.set_divider(1);
        gpt1.enable();

        FsmManager {
            key_states: initial_state,
            tick_timer: gpt1,
            last_tick: 0,
            _pedal: KeyState::default(),
        }
    }
    pub(crate) fn set_key_state(&mut self, idx: KeyIndex, new_state: KeyState) {
        self.key_states[idx] = new_state;
    }
    pub(crate) fn off(&mut self, idx: KeyIndex) {
        self.set_key_state(idx, KeyState::Off);
    }
    pub(crate) fn press(&mut self, idx: KeyIndex, timeout: KeyPressDuration) {
        self.set_key_state(idx, KeyState::Pressing { timeout });
    }
    pub(crate) fn hold(&mut self, idx: KeyIndex) {
        self.set_key_state(
            idx,
            KeyState::Holding {
                timeout: Self::HOLD_TIMEOUT,
            },
        );
    }
    pub(crate) fn repeat(&mut self, idx: KeyIndex, pressing_timeout: KeyPressDuration) {
        self.set_key_state(
            idx,
            KeyState::Repeating {
                timeout: Self::REPEAT_TIMEOUT,
                pressing_timeout,
            },
        );
    }
    pub(crate) fn release(&mut self, idx: KeyIndex) {
        self.set_key_state(
            idx,
            KeyState::Releasing {
                timeout: Self::RELEASE_TIMEOUT,
            },
        );
    }
    pub(crate) fn tick(&mut self, _logger: &mut Logger) {
        let current_us = self.tick_timer.count();
        let elapsed_us = if self.tick_timer.is_rollover() {
            self.tick_timer.clear_rollover();
            self.tick_timer.reset();
            u32::MAX
                .saturating_sub(self.last_tick)
                .saturating_add(current_us)
        } else {
            current_us.saturating_sub(self.last_tick)
        };
        // Decrement timeouts and advance key states as needed
        self.last_tick = current_us;
        for key_idx in (0..NUM_KEYS).filter_map(|idx| idx.try_into().ok()) {
            match self.key_states[key_idx] {
                KeyState::Off => (),
                KeyState::Pressing { timeout } => {
                    match timeout.to_micros().saturating_sub(elapsed_us) {
                        0 => self.hold(key_idx),
                        timeout => self.set_key_state(
                            key_idx,
                            KeyState::Pressing {
                                timeout: MicrosDurationU32::micros(timeout),
                            },
                        ),
                    }
                }
                KeyState::Holding { timeout } => {
                    match timeout.to_micros().saturating_sub(elapsed_us) {
                        0 => self.set_key_state(
                            key_idx,
                            KeyState::Releasing {
                                timeout: Self::RELEASE_TIMEOUT,
                            },
                        ),
                        timeout => self.set_key_state(
                            key_idx,
                            KeyState::Holding {
                                timeout: MicrosDurationU32::micros(timeout),
                            },
                        ),
                    }
                }
                KeyState::Releasing { timeout } => {
                    match timeout.to_micros().saturating_sub(elapsed_us) {
                        0 => self.off(key_idx),
                        timeout => self.set_key_state(
                            key_idx,
                            KeyState::Releasing {
                                timeout: MicrosDurationU32::micros(timeout),
                            },
                        ),
                    }
                }
                KeyState::Repeating {
                    timeout,
                    pressing_timeout,
                } => match timeout.to_micros().saturating_sub(elapsed_us) {
                    0 => self.press(key_idx, pressing_timeout),
                    timeout => self.set_key_state(
                        key_idx,
                        KeyState::Repeating {
                            timeout: MicrosDurationU32::micros(timeout),
                            pressing_timeout,
                        },
                    ),
                },
            }
        }
    }
}
