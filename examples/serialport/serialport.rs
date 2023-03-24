//! Reads data from a serial port and prints the contained sml messages to stdout

use std::io::{self, Read, Write};
use std::time::Duration;

use serialport::{Parity, SerialPortInfo, SerialPortType, StopBits, UsbPortInfo};

fn main() -> std::io::Result<()> {
    let ports = serialport::available_ports().expect("No ports found!");

    let port_name = select_port(&ports)?;

    println!("Connecting to port {}", port_name);
    let port = serialport::new(port_name, 9_600)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("Failed to open port");

    let mut decoder = sml_rs::transport::Decoder::<Vec<u8>>::new();

    for res in port.bytes() {
        let byte = res?;

        match decoder.push_byte(byte) {
            Ok(None) => {}
            Ok(Some(decoded_bytes)) => {
                println!("{:#?}", sml_rs::parser::complete::parse(decoded_bytes));
            }
            Err(e) => {
                println!("Err({:?})", e);
            }
        }
    }

    if let Some(e) = decoder.finalize() {
        println!("Err({:?})", e);
    }

    Ok(())
}

fn select_port(ports: &[SerialPortInfo]) -> std::io::Result<&String> {
    if ports.is_empty() {
        panic!("No serial ports found.");
    }

    println!("Please select a port:");
    // print available ports
    for (idx, p) in ports.iter().enumerate() {
        let prod_str = if let SerialPortType::UsbPort(UsbPortInfo {
            product: Some(prod),
            ..
        }) = &p.port_type
        {
            format!("({prod})")
        } else {
            "".to_string()
        };
        println!("  {idx}: {} {}", p.port_name, prod_str);
    }

    print!("Enter port number: ");
    std::io::stdout().flush()?;

    // let user select serial port
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line)?;
    let selected_idx: usize = input_line.trim().parse().expect("Input not an integer");

    let Some(port_name) = ports.get(selected_idx).map(|x| &x.port_name) else {
        panic!("Invalid port number.");
    };

    Ok(port_name)
}
