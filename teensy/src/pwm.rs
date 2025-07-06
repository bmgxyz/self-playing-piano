use cortex_m::asm::delay;
use embedded_hal::digital::OutputPin;
use fugit::MicrosDurationU32;
use teensy4_bsp::{
    board::IPG_FREQUENCY,
    hal::{
        flexpwm::{
            self, ClockSelect, LoadMode, Output as PwmOutput, PairOperation, Prescaler, Pwm,
            Submodules,
        },
        gpio::{Output as GpioOutput, Port},
        gpt::{ClockSource, Gpt1, Mode},
    },
    pins::{
        t41::Pins,
        tmm::{P17, P18, P19, P20, P21, P22, P23},
    },
};

use crate::{state::KeyState, KeyIndex, Logger, NUM_KEYS};

type KeyPressDuration = MicrosDurationU32;

pub(crate) struct PwmManager {
    data_clock: GpioOutput<P17>,
    clear_al: GpioOutput<P18>,
    latch: GpioOutput<P19>,
    enable_al: GpioOutput<P20>,
    kick: GpioOutput<P21>,
    hold_enable: GpioOutput<P22>,
    _hold_pwm: PwmOutput<P23>,
    tick_timer: Gpt1,
    last_tick: u32,
    pub(crate) key_states: [KeyState; NUM_KEYS],
    needs_update: bool,
    _pedal: KeyState,
}

impl PwmManager {
    const HOLD_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(30_000_000);
    const RELEASE_TIMEOUT: MicrosDurationU32 = MicrosDurationU32::micros(100_000);
    const REPEAT_TIMEOUT: MicrosDurationU32 = Self::RELEASE_TIMEOUT;

    const SERIAL_DELAY_CYCLES: u32 = 512;

    const PWM_FREQUENCY_HZ: u32 = 20_000;
    const PWM_ROLLOVER_VALUE: i16 = (IPG_FREQUENCY / Self::PWM_FREQUENCY_HZ) as i16;
    const PWM_DUTY_CYCLE: f32 = 0.18;
    const PWM_TURN_OFF_VALUE: i16 = (Self::PWM_ROLLOVER_VALUE as f32 * Self::PWM_DUTY_CYCLE) as i16;

    pub(crate) fn new(
        mut gpio1: Port<1>,
        pins: Pins,
        mut gpt1: Gpt1,
        flexpwm4: (Pwm<4>, Submodules<4>),
    ) -> Self {
        gpt1.set_clock_source(ClockSource::PeripheralClock);
        gpt1.set_mode(Mode::FreeRunning);
        gpt1.set_divider(1);
        gpt1.enable();

        let (mut pwm_control, (_, mut pwm_submodule, _, _)) = flexpwm4;

        let data_clock = gpio1.output(pins.p17);
        let clear_al = gpio1.output(pins.p18);
        let latch = gpio1.output(pins.p19);
        let enable_al = gpio1.output(pins.p20);
        let kick = gpio1.output(pins.p21);
        let hold_enable = gpio1.output(pins.p22);
        let hold_pwm = PwmOutput::new_a(pins.p23);

        pwm_submodule.set_debug_enable(true);
        pwm_submodule.set_wait_enable(true);
        pwm_submodule.set_clock_select(ClockSelect::Ipg);
        pwm_submodule.set_prescaler(Prescaler::Prescaler1);
        pwm_submodule.set_pair_operation(PairOperation::Independent);
        pwm_submodule.set_load_mode(LoadMode::reload_full());
        pwm_submodule.set_load_frequency(1);
        pwm_submodule.set_initial_count(&pwm_control, 0);
        pwm_submodule.set_value(
            flexpwm::FULL_RELOAD_VALUE_REGISTER,
            Self::PWM_ROLLOVER_VALUE,
        );

        hold_pwm.set_turn_on(&pwm_submodule, 0);
        hold_pwm.set_turn_off(&pwm_submodule, Self::PWM_TURN_OFF_VALUE);
        hold_pwm.set_output_enable(&mut pwm_control, true);
        pwm_submodule.set_load_ok(&mut pwm_control);
        pwm_submodule.set_running(&mut pwm_control, true);

        let mut pwm_manager = PwmManager {
            data_clock,
            clear_al,
            latch,
            enable_al,
            kick,
            hold_enable,
            _hold_pwm: hold_pwm,
            key_states: [KeyState::Off; NUM_KEYS],
            needs_update: false,
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
        self.hold_enable.clear();

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
        self.needs_update |= current_state.needs_update(&new_state);
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
    fn send_update(&mut self, _logger: &mut Logger) {
        for key in self.key_states.into_iter().rev() {
            let _ = self.kick.set_state(key.get_kick_state());
            let _ = self.hold_enable.set_state(key.get_hold_state());
            self.sr_clock();
        }
        self.kick.clear();
        self.hold_enable.clear();

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
        if self.needs_update {
            self.send_update(logger);
            self.needs_update = false;
        }
    }
}
