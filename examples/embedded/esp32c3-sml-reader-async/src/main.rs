#![no_std]
#![no_main]

use core::slice;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
#[cfg(not(feature = "smart-led"))]
use esp_hal::gpio::AnyOutput;
use esp_hal::{
    clock::ClockControl,
    gpio::{Io, Level},
    peripherals::{Peripherals, UART1},
    prelude::*,
    system::SystemControl,
    timer::{timg::TimerGroup, ErasedTimer, OneShotTimer},
    uart::{
        config::{Config, StopBits},
        Uart, UartRx,
    },
    Async,
};

#[cfg(feature = "smart-led")]
use esp_hal::rmt::Rmt;
#[cfg(feature = "smart-led")]
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
#[cfg(feature = "smart-led")]
use smart_leds::RGB;

use sml_rs::{transport::Decoder, util::ArrayBuf};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks, None);
    let timer0 = OneShotTimer::new(timg0.timer0.into());
    let timers = [timer0];
    let timers = mk_static!([OneShotTimer<ErasedTimer>; 1], timers);

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

    let mut led;
    // LED Configuration
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
        .stop_bits(StopBits::STOP1)
        .rx_fifo_full_threshold(0);
    let uart1 =
        Uart::new_async_with_config(peripherals.UART1, uart_config, &clocks, tx_pin, rx_pin)
            .unwrap();
    let (_tx, rx) = uart1.split();

    // Init embassy
    log::info!("Initializing embassy...");
    esp_hal_embassy::init(&clocks, timers);

    // Test LED
    log::info!("Testing LED...");
    let mut led_state = false;
    for _ in 0..8 {
        toggle_led(&mut led_state, &mut led);
        Timer::after(Duration::from_millis(200)).await;
    }

    // Spawn print task
    log::info!("Spawning print task...");
    spawner.spawn(print()).ok();

    // Spawn reader task
    log::info!("Spawning reader task...");
    spawner.spawn(reader(rx, led)).ok();
}

#[embassy_executor::task]
async fn reader(mut rx: UartRx<'static, UART1, Async>, mut led: LedTy) {
    log::info!("Starting reader task!");
    let mut led_state = false;

    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);

    loop {
        // read byte from the pin
        let mut b = 0u8;
        let r = embedded_io_async::Read::read(&mut rx, slice::from_mut(&mut b)).await;

        // toggle the LED
        toggle_led(&mut led_state, &mut led);

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
                        log::info!("Error receiving data: {e}")
                    }
                }
            }
            Err(e) => {
                log::warn!("UART RX Error: {:?}", e)
            }
        }
    }
}

#[embassy_executor::task]
async fn print() {
    loop {
        log::info!("Hello from the print task!");
        Timer::after(Duration::from_millis(5000)).await;
    }
}

#[cfg(feature = "smart-led")]
type LedTy = SmartLedsAdapter<esp_hal::rmt::Channel<esp_hal::Blocking, 0>, 25>;
#[cfg(not(feature = "smart-led"))]
type LedTy = AnyOutput<'static>;

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

fn toggle_led(state: &mut bool, led: &mut LedTy) {
    *state = !*state;
    led_write(led, (*state).into())
}
