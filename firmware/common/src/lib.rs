#![no_std]

pub const TWI_BASE_ADDR: u8 = 0x50;

pub const NUM_KEYS: usize = 88;
pub const NUM_KEYS_PER_MODULE: usize = 11;
pub const NUM_MODULES: usize = NUM_KEYS / NUM_KEYS_PER_MODULE;

#[derive(Debug, PartialEq)]
pub struct ModuleKeyIndex(u8);

#[derive(Debug)]
pub struct InvalidModuleKeyIndex;

impl TryFrom<u8> for ModuleKeyIndex {
    type Error = InvalidModuleKeyIndex;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            idx if idx < NUM_KEYS_PER_MODULE as u8 => Ok(ModuleKeyIndex(idx)),
            _ => Err(InvalidModuleKeyIndex),
        }
    }
}

impl From<ModuleKeyIndex> for u8 {
    fn from(value: ModuleKeyIndex) -> Self {
        value.0
    }
}

impl From<ModuleKeyIndex> for usize {
    fn from(value: ModuleKeyIndex) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ModuleKeyState {
    Off,
    Holding,
    Pressing,
}

#[derive(Debug)]
pub struct InvalidModuleKeyState;

impl TryFrom<u8> for ModuleKeyState {
    type Error = InvalidModuleKeyState;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ModuleKeyState::Off),
            1 => Ok(ModuleKeyState::Holding),
            2 => Ok(ModuleKeyState::Pressing),
            _ => Err(InvalidModuleKeyState),
        }
    }
}

impl From<ModuleKeyState> for u8 {
    fn from(value: ModuleKeyState) -> Self {
        match value {
            ModuleKeyState::Off => 0,
            ModuleKeyState::Holding => 1,
            ModuleKeyState::Pressing => 2,
        }
    }
}

#[derive(Debug)]
pub enum InvalidCommand {
    InvalidModuleKeyIndex,
    InvalidModuleKeyState,
    BadChecksum,
}

impl From<InvalidModuleKeyIndex> for InvalidCommand {
    fn from(_value: InvalidModuleKeyIndex) -> Self {
        InvalidCommand::InvalidModuleKeyIndex
    }
}

impl From<InvalidModuleKeyState> for InvalidCommand {
    fn from(_value: InvalidModuleKeyState) -> Self {
        InvalidCommand::InvalidModuleKeyState
    }
}

#[derive(Debug, PartialEq)]
pub struct Command {
    pub key_idx: ModuleKeyIndex,
    pub new_state: ModuleKeyState,
}

impl TryFrom<u8> for Command {
    type Error = InvalidCommand;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let key_idx: ModuleKeyIndex = (value >> 5).try_into()?;
        let new_state: ModuleKeyState = ((value >> 3) % (1 << 2)).try_into()?;
        if value.count_ones() % 4 == 0 {
            Ok(Command { key_idx, new_state })
        } else {
            Err(InvalidCommand::BadChecksum)
        }
    }
}

impl From<&Command> for u8 {
    fn from(value: &Command) -> Self {
        const CHECKSUM_LUT: [u8; 6] = [0b0, 0b111, 0b11, 0b1, 0b0, 0b111];
        let key_idx = value.key_idx.0 << 5;
        let new_state = <ModuleKeyState as Into<u8>>::into(value.new_state) << 3;
        let checksum = CHECKSUM_LUT[(key_idx.count_ones() + new_state.count_ones()) as usize];
        key_idx | new_state | checksum
    }
}

#[test]
fn encode_decode_all() {
    for key_idx in 0..8 {
        for key_state in 0..2 {
            let command = Command {
                key_idx: ModuleKeyIndex(key_idx),
                new_state: key_state.try_into().unwrap(),
            };
            let byte: u8 = (&command).into();
            let new_command: Command = byte.try_into().unwrap();
            assert_eq!(command, new_command);
        }
    }
}
