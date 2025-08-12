use common::{Command, NUM_KEYS};
use core::fmt::Write;
use cortex_m::asm::delay;
use embedded_hal::{digital::OutputPin, i2c::I2c};
use fugit::MicrosDurationU32;
use heapless::spsc::Queue;
use teensy4_bsp::{
    board::Lpi2c1,
    hal::{
        gpio::Output,
        gpt::{ClockSource, Gpt1, Mode},
    },
    pins::tmm::P20,
};

use crate::{
    index::{KeyIndex, ModuleIndex},
    log::Logger,
    state::KeyState,
    warn,
};

pub(crate) struct PwmManager {
    tick_timer: Gpt1,
    last_tick: u32,
    key_states: [KeyState; NUM_KEYS],
    i2c: Lpi2c1,
    pwm_enable: Output<P20>,
    updates: Queue<(ModuleIndex, Command), 128>,
    _pedal: KeyState,
}

type KeyPressDuration = MicrosDurationU32;

impl PwmManager {
    const HOLD_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(30_000_000);
    const RELEASE_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
    const REPEAT_TIMEOUT: MicrosDurationU32 = Self::RELEASE_TIMEOUT;

    pub(crate) fn new(
        mut gpt1: Gpt1,
        initial_state: [KeyState; NUM_KEYS],
        i2c: Lpi2c1,
        mut pwm_enable: Output<P20>,
    ) -> Self {
        gpt1.set_clock_source(ClockSource::PeripheralClock);
        gpt1.set_mode(Mode::FreeRunning);
        gpt1.set_divider(1);
        gpt1.enable();

        let _ = pwm_enable.set_high();

        PwmManager {
            key_states: initial_state,
            tick_timer: gpt1,
            last_tick: 0,
            i2c,
            pwm_enable,
            updates: Queue::new(),
            _pedal: KeyState::default(),
        }
    }
    pub(crate) fn get_key_state(&self, idx: KeyIndex) -> KeyState {
        self.key_states[idx]
    }
    pub(crate) fn set_key_state(&mut self, idx: KeyIndex, new_state: KeyState) {
        let current_state = self.get_key_state(idx);
        self.key_states[idx] = new_state;
        if !current_state.same_state(&new_state) {
            if let Ok(key_idx) = idx.try_into() {
                let _ = self.updates.enqueue((
                    <KeyIndex as Into<ModuleIndex>>::into(idx),
                    Command {
                        key_idx,
                        new_state: new_state.into(),
                    },
                ));
            }
        }
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
    pub(crate) fn tick(&mut self, logger: &mut Logger) {
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
        while let Some((module_idx, command)) = self.updates.dequeue() {
            let _ = self.pwm_enable.set_low();
            delay(75_000);
            let address = module_idx.as_twi_address();
            let payload = (&command).into();
            if let Err(e) = self.i2c.write(address, &[payload]) {
                warn!(
                    logger,
                    "Failed to send update to address 0x{:02x}: {:?}, {:?}", address, command, e
                );
            }
        }
        let _ = self.pwm_enable.set_high();
    }
}
