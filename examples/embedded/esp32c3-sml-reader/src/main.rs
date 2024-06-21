#![no_std]
#![no_main]

use core::slice;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{AnyOutput, Io, Level},
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    uart::{
        config::{Config, StopBits},
        Uart,
    },
};

use sml_rs::{transport::Decoder, util::ArrayBuf};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // LED
    let mut led = AnyOutput::new(io.pins.gpio18, Level::High);

    // UART PORT
    let tx_pin = io.pins.gpio1;
    let rx_pin = io.pins.gpio9;
    // let pins = TxRxPins::new_tx_rx(tx_pin, rx_pin);

    let uart_config = Config::default()
        .baudrate(9600)
        .parity_none()
        .stop_bits(StopBits::STOP1);
    let mut uart1 = Uart::new_with_config(
        peripherals.UART1,
        uart_config,
        &clocks,
        None,
        tx_pin,
        rx_pin,
    )
    .unwrap();

    esp_println::logger::init_logger_from_env();

    log::info!("Testing LED...");
    for _ in 0..4 {
        led.set_high();
        delay.delay(200.millis());
        led.set_low();
        delay.delay(200.millis());
    }

    log::info!("Reading SML messages...");

    // read_blocking(&mut led, &mut uart1)
    // Not currently implemented in esp-hal, see https://github.com/esp-rs/esp-hal/issues/1620
    read_polling(&mut led, &mut uart1)
}

#[allow(unused)]
fn read_blocking(led: &mut AnyOutput, pin: &mut impl embedded_io::Read) -> ! {
    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);

    let mut led_toggle = false;

    loop {
        let mut b = 0u8;
        // read byte from the pin
        pin.read(slice::from_mut(&mut b)).unwrap();

        led.toggle();
        led_toggle = !led_toggle;

        match decoder.push_byte(b) {
            Ok(None) => {
                continue;
            }
            Ok(Some(bytes)) => {
                log::info!("Got data: {bytes:?}")
            }
            Err(e) => {
                log::error!("Error receiving data: {e}")
            }
        }
    }
}

#[allow(unused)]
fn read_polling<PIN: embedded_io::Read + embedded_io::ReadReady>(
    led: &mut AnyOutput,
    pin: &mut PIN,
) -> ! {
    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);

    loop {
        if pin.read_ready().unwrap() {
            let mut b = 0u8;
            // read byte from the pin

            pin.read(slice::from_mut(&mut b)).unwrap();

            match decoder.push_byte(b) {
                Ok(None) => {
                    continue;
                }
                Ok(Some(bytes)) => {
                    log::info!("Got data: {bytes:?}")
                }
                Err(e) => {
                    log::error!("Error receiving data: {e}")
                }
            }
        }
        led.toggle();
    }
}
