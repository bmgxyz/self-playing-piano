use std::{
    error::Error,
    path::PathBuf,
    sync::mpsc::channel,
    thread::sleep,
    time::{Duration, Instant},
};

use clap::Parser;
use common::{
    ACK_RESPONSE, KeyIndex, KeyState, KeyVelocity, ModuleIndex, NUM_KEYS, SERIAL_BAUD_RATE,
    Schedule,
};
use midir::{Ignore, MidiInput, os::unix::VirtualInput};
use serialport::SerialPort;
use wmidi::{Channel, MidiMessage, Note, U7};

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
    let mut schedule_bytes = schedule.serialize()?.to_vec();
    schedule_bytes.extend_from_slice(b"\r\n");
    port.write_all(&schedule_bytes)?;
    Ok(())
}

fn read_response(port: &mut Box<dyn SerialPort>) -> Result<bool, Box<dyn Error>> {
    let mut buf = [0u8; 3];
    port.read_exact(&mut buf)?;
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

#[derive(Parser, Debug)]
struct Args {
    #[arg(short = '0', long)]
    driver_0: Option<PathBuf>,

    #[arg(short = '1', long)]
    driver_1: Option<PathBuf>,

    #[arg(short = '2', long)]
    driver_2: Option<PathBuf>,

    #[arg(short = '3', long)]
    driver_3: Option<PathBuf>,

    #[arg(short = '4', long)]
    driver_4: Option<PathBuf>,

    #[arg(short = '5', long)]
    driver_5: Option<PathBuf>,

    #[arg(short = '6', long)]
    driver_6: Option<PathBuf>,

    #[arg(short = '7', long)]
    driver_7: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let driver_paths = [
        args.driver_0,
        args.driver_1,
        args.driver_2,
        args.driver_3,
        args.driver_4,
        args.driver_5,
        args.driver_6,
        args.driver_7,
    ];
    let mut drivers = driver_paths
        .iter()
        .enumerate()
        .map(|(idx, dp)| match dp {
            Some(d) => match d.to_str() {
                Some(path_str) => match init_port(path_str) {
                    Ok(port) => Some(port),
                    Err(e) => {
                        eprintln!("Failed to initialize driver {idx}: {e}");
                        None
                    }
                },
                None => {
                    eprintln!("Failed to convert path for driver {idx} to str");
                    None
                }
            },
            None => {
                eprintln!("No path specified for driver {idx}, skipping");
                None
            }
        })
        .collect::<Vec<Option<Box<dyn SerialPort>>>>();
    if drivers.iter().all(|d| d.is_none()) {
        return Err("No drivers found, exiting".into());
    }

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
    let mut schedules = [
        Schedule::build(&ModuleIndex::const_new::<0>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<1>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<2>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<3>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<4>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<5>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<6>(), &key_states),
        Schedule::build(&ModuleIndex::const_new::<7>(), &key_states),
    ];
    let mut first_loop = true;

    loop {
        let loop_start = Instant::now();
        while let Ok(midi_bytes) = midi_rx.try_recv() {
            if let Ok(midi_message) = MidiMessage::try_from(midi_bytes.as_slice()) {
                match midi_message {
                    MidiMessage::NoteOff(MIDI_CHANNEL, note, _)
                    | MidiMessage::NoteOn(MIDI_CHANNEL, note, U7::MIN) => {
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
        for (idx, (maybe_port, old_schedule)) in
            drivers.iter_mut().zip(schedules.iter_mut()).enumerate()
        {
            if let Some(port) = maybe_port {
                let new_schedule =
                    Schedule::build(&ModuleIndex::new_saturating(idx as u8), &key_states);
                if new_schedule != *old_schedule || first_loop {
                    *old_schedule = new_schedule;
                    send_schedule(port, old_schedule)?;
                    if let Ok(resp) = read_response(port)
                        && !resp
                    {
                        eprintln!("Got NAK from driver {idx}");
                    }
                }
            }
        }
        first_loop = false;
        if loop_start.elapsed() < LOOP_DELAY {
            sleep(loop_start + LOOP_DELAY - Instant::now());
        }
    }
}
