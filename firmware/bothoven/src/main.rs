use std::{error::Error, time::Duration};

use common::{
    ACK_RESPONSE, KeyState, KeyVelocity, NUM_KEYS_PER_MODULE, SERIAL_BAUD_RATE, Schedule,
    build_schedule,
};
use serialport::SerialPort;

fn init_port(path: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    let mut port = serialport::new(path, SERIAL_BAUD_RATE)
        .timeout(Duration::from_millis(100))
        .open()?;
    std::thread::sleep(Duration::from_millis(1500));
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut key_states = [KeyState::Off; NUM_KEYS_PER_MODULE];
    key_states[0] = KeyState::Pressing {
        timeout_us: 100_000,
        velocity: KeyVelocity::const_new::<32>(),
    };
    let mut schedule = build_schedule(&key_states);
    let mut port = init_port("/dev/ttyUSB0")?;
    send_schedule(&mut port, &schedule)?;
    if let Ok(resp) = read_response(&mut port)
        && !resp
    {
        println!("nak");
    }
    loop {
        for key_state in key_states.iter_mut() {
            // TODO should probably switch state machine logic to millis instead of micros
            key_state.tick(LOOP_DELAY.as_micros() as u32);
        }
        let new_schedule = build_schedule(&key_states);
        if new_schedule != schedule {
            println!("{schedule}");
            schedule = new_schedule;
            send_schedule(&mut port, &schedule)?;
        }
        std::thread::sleep(LOOP_DELAY);
    }
}
