use core::ops::{Index, IndexMut};

use midi_convert::midi_types::Note;

use crate::NUM_KEYS;

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
