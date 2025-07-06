use embedded_hal::digital::PinState;
use fugit::MicrosDurationU32;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum KeyState {
    #[default]
    Off,
    Pressing {
        timeout: MicrosDurationU32,
    },
    Holding {
        timeout: MicrosDurationU32,
    },
    Repeating {
        timeout: MicrosDurationU32,
        pressing_timeout: MicrosDurationU32,
    },
    Releasing {
        timeout: MicrosDurationU32,
    },
}

impl KeyState {
    pub(crate) fn needs_update(&self, other: &KeyState) -> bool {
        self.get_kick_state() != other.get_kick_state()
            || self.get_hold_state() != other.get_hold_state()
    }
    pub(crate) fn get_kick_state(&self) -> PinState {
        match self {
            KeyState::Off => PinState::Low,
            KeyState::Pressing { .. } => PinState::High,
            KeyState::Holding { .. } => PinState::Low,
            KeyState::Repeating { .. } => PinState::Low,
            KeyState::Releasing { .. } => PinState::Low,
        }
    }
    pub(crate) fn get_hold_state(&self) -> PinState {
        match self {
            KeyState::Off => PinState::Low,
            KeyState::Pressing { .. } => PinState::Low,
            KeyState::Holding { .. } => PinState::High,
            KeyState::Repeating { .. } => PinState::Low,
            KeyState::Releasing { .. } => PinState::Low,
        }
    }
}
