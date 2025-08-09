use core::ops::{Index, IndexMut};

use midi_convert::midi_types::Note;

use crate::NUM_KEYS;

#[derive(Debug)]
pub(crate) struct InvalidModuleIndex;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleIndex(usize);

impl TryFrom<usize> for ModuleIndex {
    type Error = InvalidModuleIndex;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            idx if idx < Self::NUM_MODULES => Ok(ModuleIndex(idx)),
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
        (idx / Self::KEYS_PER_MODULE).try_into().unwrap()
    }
}

impl ModuleIndex {
    const KEYS_PER_MODULE: usize = 8;
    pub(crate) const NUM_MODULES: usize = NUM_KEYS.div_ceil(Self::KEYS_PER_MODULE);

    pub(crate) fn as_key_indices(&self) -> [KeyIndex; Self::KEYS_PER_MODULE] {
        let mut indices = [0usize.try_into().unwrap(); Self::KEYS_PER_MODULE];
        let module_idx_usize: usize = (*self).into();
        let start = module_idx_usize * Self::KEYS_PER_MODULE;
        let end = start + Self::KEYS_PER_MODULE;
        for idx in start..end {
            // indices[idx] = idx.try_into().unwrap();
        }
        indices
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct KeyIndex(u8);

impl KeyIndex {
    const MIN_MIDI_PITCH: u8 = 21;
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

#[derive(Debug)]
pub(crate) struct InvalidKeyIndex;

impl TryFrom<Note> for KeyIndex {
    type Error = InvalidKeyIndex;

    fn try_from(value: Note) -> Result<Self, Self::Error> {
        let idx: u8 = value.into();
        if idx < Self::MIN_MIDI_PITCH {
            return Err(InvalidKeyIndex);
        }
        (idx - Self::MIN_MIDI_PITCH).try_into()
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
