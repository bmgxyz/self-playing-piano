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
