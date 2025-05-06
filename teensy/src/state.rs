use crate::KeyPwm;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum KeyState {
    #[default]
    Off,
    Pressing {
        timeout: u32,
        pwm: KeyPwm,
    },
    Holding {
        timeout: u32,
    },
    Repeating {
        timeout: u32,
        /// This is *not* the PWM duty cycle of the repeating state itself.
        /// Instead, this state carries the desired PWM duty cycle for the
        /// pressing state that this state will eventually transition to. The
        /// true duty cycle of this state is zero.
        pwm: KeyPwm,
    },
    Releasing {
        timeout: u32,
    },
}

impl KeyState {
    pub(crate) fn same_duty_cycle(&self, other: &KeyState) -> bool {
        let pwm_self: KeyPwm = (*self).into();
        let pwm_other = (*other).into();
        pwm_self == pwm_other
    }
}
