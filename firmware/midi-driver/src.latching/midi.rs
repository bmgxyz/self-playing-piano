use core::fmt::Write;
use fugit::MicrosDurationU32;
use midi_convert::{
    midi_types::{Channel, MidiMessage, Note, Value7},
    parse::MidiTryParseSlice,
};
use usbd_midi::UsbMidiEventPacket;

use crate::{debug, warn, FsmManager, KeyState, Logger};

const MIDI_CHANNEL: Channel = Channel::C1;

fn velocity_to_duration(velocity: Value7) -> MicrosDurationU32 {
    MicrosDurationU32::millis(<Value7 as Into<u8>>::into(velocity) as u32 / 2)
}

pub(crate) fn handle_midi_packet(
    logger: &mut Logger,
    packet: UsbMidiEventPacket,
    pwm_manager: &mut FsmManager,
) {
    let message = match MidiMessage::try_parse_slice(packet.payload_bytes()) {
        Ok(m) => m,
        Err(e) => {
            warn!(logger, "Failed to parse MIDI packet {packet:?}: {e:?}");
            return;
        }
    };
    match message {
        MidiMessage::NoteOn(channel, note, velocity) => {
            if channel != MIDI_CHANNEL {
                return;
            }
            debug!(
                logger,
                "MIDI ON {note:?} ({}), {velocity:?}",
                <Note as Into<u8>>::into(note)
            );
            if let Ok(key_idx) = note.try_into() {
                let key_state = pwm_manager.key_states[key_idx];
                match key_state {
                    KeyState::Off => {
                        let duration = velocity_to_duration(velocity);
                        pwm_manager.press(key_idx, duration)
                    }
                    KeyState::Holding { .. } | KeyState::Releasing { .. } => {
                        pwm_manager.repeat(key_idx, velocity_to_duration(velocity));
                    }
                    KeyState::Pressing { .. } | KeyState::Repeating { .. } => (),
                }
            }
        }
        MidiMessage::NoteOff(channel, note, _) => {
            if channel != MIDI_CHANNEL {
                return;
            }
            debug!(
                logger,
                "MIDI OFF {note:?} ({})",
                <Note as Into<u8>>::into(note)
            );
            if let Ok(key_idx) = note.try_into() {
                let key_state = pwm_manager.key_states[key_idx];
                match key_state {
                    KeyState::Pressing { .. }
                    | KeyState::Holding { .. }
                    | KeyState::Repeating { .. } => pwm_manager.release(key_idx),
                    KeyState::Off | KeyState::Releasing { .. } => (),
                }
            }
        }
        _ => (),
    }
}
