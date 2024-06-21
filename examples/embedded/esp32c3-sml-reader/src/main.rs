#![no_std]
#![no_main]

use core::slice;

use esp_backtrace as _;
#[cfg(not(feature = "smart-led"))]
use esp_hal::gpio::AnyOutput;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Io, Level},
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    uart::{
        config::{Config, StopBits},
        Uart,
    },
};

#[cfg(feature = "smart-led")]
use esp_hal::rmt::Rmt;
#[cfg(feature = "smart-led")]
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
#[cfg(feature = "smart-led")]
use smart_leds::RGB;

use sml_rs::{transport::Decoder, util::ArrayBuf};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // -----------------------------------------------------------------------
    // Pin configuration - adapt to your board
    // -----------------------------------------------------------------------
    let led_pin = io.pins.gpio8;
    // tx is unused, but needs to be provided for UART
    let tx_pin = io.pins.gpio10;
    let rx_pin = io.pins.gpio9;
    // -----------------------------------------------------------------------
    // -----------------------------------------------------------------------
    // -----------------------------------------------------------------------

    // Init logging
    esp_println::logger::init_logger_from_env();

    // LED Configuration
    let mut led;
    #[cfg(feature = "smart-led")]
    {
        let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
        let rmt_buffer = smartLedBuffer!(1);
        led = SmartLedsAdapter::new(rmt.channel0, led_pin, rmt_buffer, &clocks);
    }
    #[cfg(not(feature = "smart-led"))]
    {
        led = AnyOutput::new(led_pin, Level::High);
    }

    // UART Configuration
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

    // Closure toggling the LED
    let mut led_state = false;
    let mut toggle_led = || {
        led_state = !led_state;
        led_write(&mut led, led_state.into());
    };

    // Test LED
    log::info!("Testing LED...");
    for _ in 0..8 {
        toggle_led();
        delay.delay(200.millis());
    }

    log::info!("Reading SML messages...");

    #[cfg(not(feature = "polling"))]
    {
        read_blocking(&mut uart1, toggle_led)
    }
    #[cfg(feature = "polling")]
    {
        read_polling(&mut uart1, toggle_led)
    }
}

#[allow(unused)]
fn read_blocking(pin: &mut impl embedded_io::Read, mut toggle_led: impl FnMut()) -> ! {
    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);

    loop {
        // read byte from the pin
        let mut b = 0u8;
        let r = pin.read(slice::from_mut(&mut b));

        // toggle the LED
        toggle_led();

        match r {
            Ok(_) => {
                // process the read byte
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
            Err(e) => {
                log::warn!("UART RX Error: {:?}", e)
            }
        }
    }
}

#[allow(unused)]
fn read_polling<PIN: embedded_io::Read + embedded_io::ReadReady>(
    pin: &mut PIN,
    mut toggle_led: impl FnMut(),
) -> ! {
    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);

    let mut last_print_time = 0;

    // main loop
    loop {
        // reading only when data is available (non-blocking)
        if pin.read_ready().unwrap() {
            // read byte from the pin
            let mut b = 0u8;
            let r = pin.read(slice::from_mut(&mut b));

            // toggle the LED
            toggle_led();

            match r {
                Ok(_) => {
                    // process the read byte
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
                Err(e) => {
                    log::warn!("UART RX Error: {:?}", e)
                }
            }
        }

        // print a message every 5 seconds
        let mut secs_since_start = esp_hal::time::current_time()
            .duration_since_epoch()
            .to_secs();
        if secs_since_start >= last_print_time + 5 {
            last_print_time = secs_since_start;
            log::info!("Hello from the print task!");
        }
    }
}

#[cfg(feature = "smart-led")]
fn led_write<S>(led: &mut S, level: Level)
where
    S: smart_leds::SmartLedsWrite<Color = RGB<u8>>,
    S::Error: core::fmt::Debug,
{
    let color = match level {
        Level::High => RGB::new(0, 0, 2),
        Level::Low => RGB::new(0, 0, 0),
    };
    led.write([color]).unwrap()
}
#[cfg(not(feature = "smart-led"))]
fn led_write(led: &mut AnyOutput, level: Level) {
    led.set_level(level)
}
