use core::ops::{Index, IndexMut};

use common::{
    InvalidModuleKeyIndex, ModuleKeyIndex, NUM_KEYS, NUM_KEYS_PER_MODULE, NUM_MODULES,
    TWI_BASE_ADDR,
};
use midi_convert::midi_types::Note;

#[derive(Debug)]
pub(crate) struct InvalidModuleIndex;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleIndex(usize);

impl TryFrom<usize> for ModuleIndex {
    type Error = InvalidModuleIndex;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            idx if idx < NUM_MODULES => Ok(ModuleIndex(idx)),
            _ => Err(InvalidModuleIndex),
        }
    }
}

impl From<ModuleIndex> for usize {
    fn from(value: ModuleIndex) -> Self {
        value.0
    }
}

impl From<KeyIndex> for ModuleIndex {
    fn from(value: KeyIndex) -> Self {
        let idx: usize = value.into();
        (idx / NUM_KEYS_PER_MODULE).try_into().unwrap()
    }
}

impl ModuleIndex {
    pub(crate) fn as_twi_address(&self) -> u8 {
        TWI_BASE_ADDR | self.0 as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct KeyIndex(u8);

impl KeyIndex {
    const MIN_MIDI_PITCH: u8 = 21;
    const MAX_MIDI_PITCH: u8 = Self::MIN_MIDI_PITCH + NUM_KEYS as u8 - 1;
}

impl From<KeyIndex> for usize {
    fn from(value: KeyIndex) -> Self {
        value.0.into()
    }
}

impl From<KeyIndex> for u8 {
    fn from(value: KeyIndex) -> Self {
        value.0
    }
}

impl TryFrom<KeyIndex> for ModuleKeyIndex {
    type Error = InvalidModuleKeyIndex;
    fn try_from(value: KeyIndex) -> Result<Self, Self::Error> {
        (value.0 % NUM_KEYS_PER_MODULE as u8).try_into()
    }
}

#[derive(Debug)]
pub(crate) struct InvalidKeyIndex;

impl TryFrom<Note> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: Note) -> Result<Self, Self::Error> {
        let idx: u8 = value.into();
        if (Self::MIN_MIDI_PITCH..=Self::MAX_MIDI_PITCH).contains(&idx) {
            (idx - Self::MIN_MIDI_PITCH).try_into()
        } else {
            Err(InvalidKeyIndex)
        }
    }
}

impl TryFrom<usize> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let idx: u8 = match value.try_into() {
            Ok(i) => i,
            Err(_) => return Err(InvalidKeyIndex),
        };
        idx.try_into()
    }
}

impl TryFrom<u8> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < NUM_KEYS as u8 {
            Ok(KeyIndex(value))
        } else {
            Err(InvalidKeyIndex)
        }
    }
}

impl<T> Index<KeyIndex> for [T; NUM_KEYS] {
    type Output = T;
    fn index(&self, index: KeyIndex) -> &Self::Output {
        let idx: usize = index.into();
        &self[idx]
    }
}

impl<T> IndexMut<KeyIndex> for [T; NUM_KEYS] {
    fn index_mut(&mut self, index: KeyIndex) -> &mut Self::Output {
        let idx: usize = index.into();
        &mut self[idx]
    }
}
