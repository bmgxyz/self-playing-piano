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
        match (self, other) {
            (KeyState::Off, KeyState::Off) => false,
            (KeyState::Pressing { .. }, KeyState::Pressing { .. }) => false,
            (KeyState::Holding { .. }, KeyState::Holding { .. }) => false,
            (KeyState::Releasing { .. }, KeyState::Releasing { .. }) => false,
            (KeyState::Repeating { .. }, KeyState::Repeating { .. }) => false,
            _ => true,
        }
    }
}
