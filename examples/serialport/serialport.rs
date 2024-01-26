//! Reads data from a serial port and prints the contained sml messages to stdout

use serialport5::SerialPort;
use sml_rs::parser::complete::File;
use sml_rs::ReadParsedError;

fn main() -> std::io::Result<()> {
    let port = SerialPort::builder().baud_rate(9600).open("/dev/ttyUSB0")?;

    let mut reader = sml_rs::SmlReader::from_reader(port);
    loop {
        match reader.read::<File>() {
            Ok(file) => println!("{:?}", file),
            Err(ReadParsedError::IoErr(e, _)) => {
                println!("IO Error: {:?}", e);
                println!("Exiting.");
                break;
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    Ok(())
}

