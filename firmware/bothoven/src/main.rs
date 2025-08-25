use std::{
    error::Error,
    sync::mpsc::channel,
    thread::sleep,
    time::{Duration, Instant},
};

use common::{
    ACK_RESPONSE, KeyIndex, KeyState, KeyVelocity, ModuleIndex, NUM_KEYS, SERIAL_BAUD_RATE,
    Schedule,
};
use midir::{Ignore, MidiInput, os::unix::VirtualInput};
use serialport::SerialPort;
use wmidi::{Channel, MidiMessage, Note};

fn init_port(path: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    let mut port = serialport::new(path, SERIAL_BAUD_RATE)
        .timeout(Duration::from_millis(100))
        .open()?;
    sleep(Duration::from_millis(1500));
    port.set_flow_control(serialport::FlowControl::None)?;
    port.set_data_bits(serialport::DataBits::Eight)?;
    port.set_stop_bits(serialport::StopBits::One)?;
    port.set_parity(serialport::Parity::None)?;
    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(true)?;
    Ok(port)
}

fn send_schedule(
    port: &mut Box<dyn SerialPort>,
    schedule: &Schedule,
) -> Result<(), Box<dyn Error>> {
    let mut schedule_bytes = schedule.serialize()?;
    schedule_bytes.extend_from_slice(b"\r\n").unwrap(); // TODO
    port.write_all(&schedule_bytes)?;
    Ok(())
}

fn read_response(port: &mut Box<dyn SerialPort>) -> Result<bool, Box<dyn Error>> {
    let mut buf = [0u8; 3];
    port.read(&mut buf)?;
    match buf {
        ACK_RESPONSE => Ok(true),
        _ => Ok(false),
    }
}

const LOOP_DELAY: Duration = Duration::from_millis(5);
const MIDI_CHANNEL: Channel = Channel::Ch1;
const MIN_MIDI_PITCH: u8 = 21;

fn midi_note_to_key_index(note: &Note) -> Option<KeyIndex> {
    let note_idx: u8 = (*note).into();
    if let Some(n) = note_idx.checked_sub(MIN_MIDI_PITCH) {
        KeyIndex::new(n)
    } else {
        None
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (midi_tx, midi_rx) = channel();

    let mut midi_in = MidiInput::new("Bothoven")?;
    midi_in.ignore(Ignore::None);
    let _input = midi_in.create_virtual(
        "Bothoven",
        move |_timestamp, bytes, _data| {
            midi_tx.send(bytes.to_vec()).unwrap();
        },
        (),
    )?;

    let mut key_states = [KeyState::Off; NUM_KEYS];
    let mut schedule = Schedule::build(&ModuleIndex::const_new::<3>(), &key_states);
    let mut port = init_port("/dev/ttyUSB0")?;
    send_schedule(&mut port, &schedule)?;
    loop {
        let loop_start = Instant::now();
        while let Ok(midi_bytes) = midi_rx.try_recv() {
            if let Ok(midi_message) = MidiMessage::try_from(midi_bytes.as_slice()) {
                match midi_message {
                    MidiMessage::NoteOff(channel, note, _u7) if channel == MIDI_CHANNEL => {
                        if let Some(key_index) = midi_note_to_key_index(&note) {
                            key_states[key_index].midi_note_off();
                            println!(
                                "Off {: <7} {: >3} {key_index: >3}",
                                note.to_str(),
                                <Note as Into<u8>>::into(note),
                            );
                        }
                    }
                    MidiMessage::NoteOn(channel, note, u7) if channel == MIDI_CHANNEL => {
                        if let Some(key_index) = midi_note_to_key_index(&note) {
                            let velocity = KeyVelocity::new(u7.into()).unwrap();
                            key_states[key_index].midi_note_on(&velocity);
                            println!(
                                "On  {: <7} {: >3} {key_index: >3} {velocity: >3}",
                                note.to_str(),
                                <Note as Into<u8>>::into(note),
                            );
                        }
                    }
                    MidiMessage::Reset => key_states = [KeyState::Off; NUM_KEYS],
                    _ => (),
                }
            }
        }
        for key_state in key_states.iter_mut() {
            // TODO should probably switch state machine logic to millis instead of micros
            key_state.tick(LOOP_DELAY.as_micros() as u32);
        }
        let new_schedule = Schedule::build(&ModuleIndex::const_new::<3>(), &key_states);
        if new_schedule != schedule {
            schedule = new_schedule;
            send_schedule(&mut port, &schedule)?;
            if let Ok(resp) = read_response(&mut port)
                && !resp
            {
                println!("nak");
            }
        }
        if loop_start.elapsed() < LOOP_DELAY {
            sleep(loop_start + LOOP_DELAY - Instant::now());
        }
    }
}
