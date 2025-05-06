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
