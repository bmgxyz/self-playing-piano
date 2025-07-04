use core::fmt::Write;
use cortex_m::asm::delay;
use fugit::MicrosDurationU32;
use teensy4_bsp::{
    hal::{
        gpio::{Output, Port},
        gpt::{ClockSource, Gpt1, Mode},
    },
    pins::{
        t41::Pins,
        tmm::{P17, P18, P19, P20, P21, P22},
    },
};

use crate::{state::KeyState, trace, KeyIndex, Logger, NUM_KEYS};

type KeyPressDuration = MicrosDurationU32;

pub(crate) struct PwmManager {
    data_clock: Output<P17>,
    clear_al: Output<P18>,
    latch: Output<P19>,
    enable_al: Output<P20>,
    kick: Output<P21>,
    hold: Output<P22>,
    tick_timer: Gpt1,
    last_tick: u32,
    pub(crate) key_states: [KeyState; NUM_KEYS],
    need_update: bool,
    _pedal: KeyState,
}

impl PwmManager {
    const HOLD_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(30_000_000);
    const RELEASE_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
    const REPEAT_TIMEOUT: MicrosDurationU32 = Self::RELEASE_TIMEOUT;

    const SERIAL_DELAY_CYCLES: u32 = 512;

    pub(crate) fn new(mut gpio1: Port<1>, pins: Pins, mut gpt1: Gpt1) -> Self {
        gpt1.set_clock_source(ClockSource::PeripheralClock);
        gpt1.set_mode(Mode::FreeRunning);
        gpt1.set_divider(1);
        gpt1.enable();

        // let serial_timer = Blocking::from_gpt(gpt2);

        let data_clock = gpio1.output(pins.p17);
        let clear_al = gpio1.output(pins.p18);
        let latch = gpio1.output(pins.p19);
        let enable_al = gpio1.output(pins.p20);
        let kick = gpio1.output(pins.p21);
        let hold = gpio1.output(pins.p22);
        let _pwm = (); // TODO

        let mut pwm_manager = PwmManager {
            data_clock,
            clear_al,
            latch,
            enable_al,
            kick,
            hold,
            key_states: [KeyState::Off; NUM_KEYS],
            need_update: false,
            tick_timer: gpt1,
            last_tick: 0,
            _pedal: KeyState::default(),
        };

        pwm_manager.reset();
        pwm_manager
    }
    fn wait_clock(&mut self) {
        delay(Self::SERIAL_DELAY_CYCLES);
    }
    fn sr_clock(&mut self) {
        if self.data_clock.is_set() {
            self.data_clock.clear();
        }
        self.wait_clock();
        self.data_clock.set();
        self.wait_clock();
        self.data_clock.clear();
    }
    fn reset(&mut self) {
        self.data_clock.clear();
        self.latch.clear();
        self.kick.clear();
        self.hold.clear();

        self.enable_al.set();
        self.clear_al.set();
        self.sr_clock();
        self.clear_al.clear();
        self.sr_clock();
        self.clear_al.set();
        self.enable_al.clear();
        self.sr_clock();
    }
    pub(crate) fn set_key_state(&mut self, idx: KeyIndex, new_state: KeyState) {
        let current_state = self.key_states[idx];
        self.need_update |= current_state.needs_update(&new_state);
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
    fn send_update(&mut self, logger: &mut Logger) {
        for (idx, key) in self.key_states.into_iter().rev().enumerate() {
            match key {
                KeyState::Pressing { .. } => {
                    trace!(logger, "pressing {idx}");
                    self.kick.set();
                    self.hold.clear();
                    self.sr_clock();
                }
                KeyState::Holding { .. } => {
                    trace!(logger, "holding {idx}");
                    self.kick.clear();
                    self.hold.set();
                    self.sr_clock();
                }
                KeyState::Off | KeyState::Repeating { .. } | KeyState::Releasing { .. } => {
                    self.kick.clear();
                    self.hold.clear();
                    self.sr_clock();
                }
            }
        }
        self.kick.clear();
        self.hold.clear();

        self.latch.set();
        self.wait_clock();
        self.latch.clear();
        self.wait_clock();
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
        if self.need_update {
            self.send_update(logger);
            self.need_update = false;
        }
    }
}
