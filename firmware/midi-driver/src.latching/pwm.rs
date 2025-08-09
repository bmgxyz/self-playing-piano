use cortex_m::asm::delay;
use embedded_hal::digital::{OutputPin, PinState};
use teensy4_bsp::{
    board,
    hal::gpio::Output as GpioOutput,
    pins::{
        t41::{P34, P35, P36, P37, P38, P39, P40, P41},
        tmm::{P13, P14, P15, P16, P17, P18, P19, P20, P21, P22, P23, P33},
    },
};

use crate::{index::ModuleIndex, state::KeyState, NUM_KEYS};

pub(crate) enum PwmEdge {
    Rising,
    Falling,
}

pub struct PwmManager {
    pub(crate) key_states: [KeyState; NUM_KEYS],
    pub(crate) en_al: GpioOutput<P33>,
    pub(crate) b1: GpioOutput<P34>,
    pub(crate) b2: GpioOutput<P35>,
    pub(crate) b3: GpioOutput<P36>,
    pub(crate) b4: GpioOutput<P37>,
    pub(crate) b5: GpioOutput<P38>,
    pub(crate) b6: GpioOutput<P39>,
    pub(crate) b7: GpioOutput<P40>,
    pub(crate) b8: GpioOutput<P41>,
    pub(crate) clk00: GpioOutput<P13>,
    pub(crate) clk01: GpioOutput<P14>,
    pub(crate) clk02: GpioOutput<P15>,
    pub(crate) clk03: GpioOutput<P16>,
    pub(crate) clk04: GpioOutput<P17>,
    pub(crate) clk05: GpioOutput<P18>,
    pub(crate) clk06: GpioOutput<P19>,
    pub(crate) clk07: GpioOutput<P20>,
    pub(crate) clk08: GpioOutput<P21>,
    pub(crate) clk09: GpioOutput<P22>,
    pub(crate) clk10: GpioOutput<P23>,
}

impl PwmManager {
    const PWM_FREQUENCY_HZ: u32 = 20_000;
    const PWM_HOLD_DUTY_CYCLE: f32 = 0.18;

    pub(crate) const PWM_FALLING_COUNT: i16 =
        ((board::IPG_FREQUENCY / Self::PWM_FREQUENCY_HZ) as f32 * Self::PWM_HOLD_DUTY_CYCLE) as i16;
    pub(crate) const PWM_RESET_COUNT: i16 = (board::IPG_FREQUENCY / Self::PWM_FREQUENCY_HZ) as i16;
    const LATCH_DELAY_CYCLES: u32 = 512;

    fn set_bus(&mut self, data: [PinState; 8]) {
        let _ = self.b1.set_state(data[0]);
        let _ = self.b2.set_state(data[1]);
        let _ = self.b3.set_state(data[2]);
        let _ = self.b4.set_state(data[3]);
        let _ = self.b5.set_state(data[4]);
        let _ = self.b6.set_state(data[5]);
        let _ = self.b7.set_state(data[6]);
        let _ = self.b8.set_state(data[7]);
    }
    fn set_clock(&mut self, module_idx: &ModuleIndex, state: PinState) {
        let _ = match (*module_idx).into() {
            0 => self.clk00.set_state(state),
            1 => self.clk01.set_state(state),
            2 => self.clk02.set_state(state),
            3 => self.clk03.set_state(state),
            4 => self.clk04.set_state(state),
            5 => self.clk05.set_state(state),
            6 => self.clk06.set_state(state),
            7 => self.clk07.set_state(state),
            8 => self.clk08.set_state(state),
            9 => self.clk09.set_state(state),
            10 => self.clk10.set_state(state),
            _ => unreachable!(),
        };
    }
    fn set_all_clocks(&mut self, state: PinState) {
        let _ = self.clk00.set_state(state);
        let _ = self.clk01.set_state(state);
        let _ = self.clk02.set_state(state);
        let _ = self.clk03.set_state(state);
        let _ = self.clk04.set_state(state);
        let _ = self.clk05.set_state(state);
        let _ = self.clk06.set_state(state);
        let _ = self.clk07.set_state(state);
        let _ = self.clk08.set_state(state);
        let _ = self.clk09.set_state(state);
        let _ = self.clk10.set_state(state);
    }
    fn wait_clock(&self) {
        delay(Self::LATCH_DELAY_CYCLES);
    }
    fn pulse_clock(&mut self, module_idx: &ModuleIndex) {
        self.set_clock(&module_idx, PinState::Low);
        self.wait_clock();
        self.set_clock(&module_idx, PinState::High);
        self.wait_clock();
        self.set_clock(&module_idx, PinState::Low);
    }
    pub(crate) fn reset(&mut self) {
        self.set_bus([PinState::Low; 8]);
        self.set_all_clocks(PinState::Low);
        self.wait_clock();
        self.set_all_clocks(PinState::High);
        self.wait_clock();
        self.set_all_clocks(PinState::Low);
        let _ = self.en_al.set_low();
    }
    fn send_update(&mut self, module_idx: &ModuleIndex, data: [PinState; 8]) {
        self.set_bus(data);
        self.pulse_clock(module_idx);
    }
    pub(crate) fn handle_pwm(
        &mut self,
        key_states: &[KeyState; NUM_KEYS],
        modules_to_update: &mut [bool; ModuleIndex::NUM_MODULES],
        edge: PwmEdge,
    ) {
        // for each module...
        for (idx, needs_update) in modules_to_update.iter_mut().enumerate() {
            // ...if any of the module's keys need to be updated...
            if *needs_update {
                // ...then update them
                let module_idx: ModuleIndex = idx.try_into().unwrap();
                let data = module_idx
                    .as_key_indices()
                    .map(|key_idx| match key_states[key_idx] {
                        KeyState::Off | KeyState::Repeating { .. } | KeyState::Releasing { .. } => {
                            PinState::Low
                        }
                        KeyState::Holding { .. } => match edge {
                            PwmEdge::Rising => PinState::High,
                            PwmEdge::Falling => PinState::Low,
                        },
                        KeyState::Pressing { .. } => PinState::High,
                    });
                self.send_update(&module_idx, data);
                *needs_update = false;
            }
        }
    }
}
