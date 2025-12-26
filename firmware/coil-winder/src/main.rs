#![no_std]
#![no_main]

use arduino_hal::{delay_ms, delay_us, pins, Peripherals};
use atmega_hal::port::{mode::Output, Pin, PD3};
use embedded_hal::digital::{OutputPin, PinState};
use panic_halt as _;

const NUM_TURNS: u32 = 280;
const NUM_STEPS_PER_TURN: u32 = 800;
const NUM_STEPS: u32 = NUM_TURNS * NUM_STEPS_PER_TURN;

struct State {
    motor: MotorState,
    forward_button: ButtonState,
    backward_button: ButtonState,
}

#[derive(Clone, Copy, Debug)]
enum MotorState {
    ForwardFast,
    ForwardSlow,
    Stopped,
    BackwardSlow,
    BackwardFast,
}

impl MotorState {
    fn blocking_step(&self, step_idx: &mut u32, step_pin: &mut Pin<Output, PD3>) {
        match self {
            MotorState::ForwardSlow | MotorState::ForwardFast => {
                step_pin.set_high();
                *step_idx = step_idx.saturating_add(1);
            }
            MotorState::BackwardSlow | MotorState::BackwardFast => {
                step_pin.set_high();
                *step_idx = step_idx.saturating_sub(1);
            }
            MotorState::Stopped => (),
        }
        delay_us(500);
        step_pin.set_low();
        delay_us(match self {
            MotorState::ForwardFast => 625,
            MotorState::ForwardSlow => 4500,
            MotorState::Stopped => 0,
            MotorState::BackwardSlow => 4500,
            MotorState::BackwardFast => 625,
        });
    }
    fn forward(&mut self) {
        *self = match self {
            MotorState::ForwardFast => MotorState::ForwardFast,
            MotorState::ForwardSlow => MotorState::ForwardFast,
            MotorState::Stopped => MotorState::ForwardSlow,
            MotorState::BackwardSlow => MotorState::Stopped,
            MotorState::BackwardFast => MotorState::BackwardSlow,
        };
    }
    fn backward(&mut self) {
        *self = match self {
            MotorState::ForwardFast => MotorState::ForwardSlow,
            MotorState::ForwardSlow => MotorState::Stopped,
            MotorState::Stopped => MotorState::BackwardSlow,
            MotorState::BackwardSlow => MotorState::BackwardFast,
            MotorState::BackwardFast => MotorState::BackwardFast,
        };
    }
}

impl From<MotorState> for PinState {
    fn from(value: MotorState) -> Self {
        match value {
            MotorState::ForwardFast => PinState::Low,
            MotorState::ForwardSlow => PinState::Low,
            MotorState::Stopped => PinState::Low,
            MotorState::BackwardSlow => PinState::High,
            MotorState::BackwardFast => PinState::High,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ButtonState {
    Idle,
    Debounce(u8),
    Pressed,
    Acknowledged,
}

impl ButtonState {
    const DEBOUNCE_WAIT: u8 = 4;

    fn update(&mut self, is_triggered: bool) {
        match *self {
            ButtonState::Idle => {
                if is_triggered {
                    *self = ButtonState::Debounce(ButtonState::DEBOUNCE_WAIT);
                }
            }
            ButtonState::Debounce(d) => {
                let d_next = d.saturating_sub(1);
                if d_next == 0 {
                    if is_triggered {
                        *self = ButtonState::Pressed;
                    } else {
                        *self = ButtonState::Idle;
                    }
                } else {
                    *self = ButtonState::Debounce(d_next);
                }
            }
            ButtonState::Pressed => (),
            ButtonState::Acknowledged => {
                if !is_triggered {
                    *self = ButtonState::Idle
                }
            }
        }
    }
    fn is_pressed(&self) -> bool {
        match self {
            ButtonState::Idle | ButtonState::Debounce(_) | ButtonState::Acknowledged => false,
            ButtonState::Pressed => true,
        }
    }
    fn acknowledge(&mut self) {
        if let ButtonState::Pressed = *self {
            *self = ButtonState::Acknowledged;
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let pins = pins!(dp);

    let mut dir = pins.d2.into_output();
    let mut step = pins.d3.into_output();
    let _sleep_al = pins.d4.into_output_high();
    let mut reset_al = pins.d5.into_output_high();
    let _ms3 = pins.d6.into_output();
    let _ms2 = pins.d7.into_output_high();
    let _ms1 = pins.d8.into_output();
    let _enable_al = pins.d9.into_output();
    let forward_al = pins.d10.into_pull_up_input();
    let backward_al = pins.d11.into_pull_up_input();
    let mut led = pins.d13.into_output();

    let mut state = State {
        motor: MotorState::Stopped,
        forward_button: ButtonState::Idle,
        backward_button: ButtonState::Idle,
    };

    reset_al.set_low();
    delay_ms(10);
    reset_al.set_high();

    let mut step_idx = 0;

    while step_idx < NUM_STEPS {
        state.forward_button.update(forward_al.is_low());
        if state.forward_button.is_pressed() {
            state.forward_button.acknowledge();
            state.motor.forward();
            let _ = dir.set_state(state.motor.into());
        }

        state.backward_button.update(backward_al.is_low());
        if state.backward_button.is_pressed() {
            state.backward_button.acknowledge();
            state.motor.backward();
            let _ = dir.set_state(state.motor.into());
        }

        state.motor.blocking_step(&mut step_idx, &mut step);
    }

    led.set_high();
    loop {}
}
