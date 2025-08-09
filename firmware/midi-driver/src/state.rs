use common::ModuleKeyState;
use fugit::MicrosDurationU32;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum KeyState {
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
    pub(crate) fn same_state(&self, other: &KeyState) -> bool {
        matches!(
            (self, other),
            (KeyState::Off, KeyState::Off)
                | (KeyState::Pressing { .. }, KeyState::Pressing { .. })
                | (KeyState::Holding { .. }, KeyState::Holding { .. })
                | (KeyState::Releasing { .. }, KeyState::Releasing { .. })
                | (KeyState::Repeating { .. }, KeyState::Repeating { .. })
        )
    }
}

impl From<KeyState> for ModuleKeyState {
    fn from(value: KeyState) -> Self {
        match value {
            KeyState::Off => ModuleKeyState::Off,
            KeyState::Pressing { .. } => ModuleKeyState::Pressing,
            KeyState::Holding { .. } => ModuleKeyState::Holding,
            KeyState::Repeating { .. } => ModuleKeyState::Off,
            KeyState::Releasing { .. } => ModuleKeyState::Off,
        }
    }
}
