#![no_std]
#![no_main]

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
use hex_literal::hex;

#[cfg(feature = "smart-led")]
use esp_hal::rmt::Rmt;
#[cfg(feature = "smart-led")]
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
#[cfg(feature = "smart-led")]
use smart_leds::RGB;

use embedded_io::Write;

// test data taken from https://github.com/devZer0/libsml-testing/blob/master/ISKRA_MT631-D1A52-K0z-H01_with_PIN.hex
const TEST_DATA: &[&[u8]] = &[
    &hex!("1B1B1B1B010101017605099DFAAC6200620072630101760101050334A8E40B0A0149534B00047A5544726201650334A737620163D0BB007605099DFAAD620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650334A737757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF650137C0C70177070100020800FF0101621E52FF62000177070100100700FF0101621B52005300C101010163F376007605099DFAAE620062007263020171016342DD001B1B1B1B1A0038EB"),
    &hex!("1B1B1B1B010101017605099DFAAF6200620072630101760101050334A8E50B0A0149534B00047A5544726201650334A73862016303AF007605099DFAB0620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650334A738757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF650137C0C80177070100020800FF0101621E52FF62000177070100100700FF0101621B52005300C201010163EA6F007605099DFAB162006200726302017101634BB0001B1B1B1B1A000705"),
    &hex!("1B1B1B1B010101017605099DFAB26200620072630101760101050334A8E60B0A0149534B00047A5544726201650334A739620163DC95007605099DFAB3620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650334A739757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF650137C0C80177070100020800FF0101621E52FF62000177070100100700FF0101621B52005300C101010163B703007605099DFAB462006200726302017101638FBB001B1B1B1B1A00D07B"),
    &hex!("1B1B1B1B010101017605099DFAB56200620072630101760101050334A8E70B0A0149534B00047A5544726201650334A73A620163AE9F007605099DFAB6620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650334A73A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF650137C0C90177070100020800FF0101621E52FF62000177070100100700FF0101621B52005300C20101016324B6007605099DFAB762006200726302017101633C45001B1B1B1B1A00C2C5"),
    &hex!("1B1B1B1B010101017605099DFAB86200620072630101760101050334A8E80B0A0149534B00047A5544726201650334A73B620163B08E007605099DFAB9620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650334A73B757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF650137C0C90177070100020800FF0101621E52FF62000177070100100700FF0101621B52005300C201010163EAD0007605099DFABA620062007263020171016352F2001B1B1B1B1A00382E"),
];

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
    let led_pin = io.pins.gpio18;
    let tx_pin = io.pins.gpio9;
    // rx is unused, but needs to be provided for UART
    let rx_pin = io.pins.gpio10;
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

    // Main Loop
    let mut data_iter = TEST_DATA.iter().cycle();
    loop {
        let data = data_iter.next().unwrap();
        log::info!("Sending data!");
        led_write(&mut led, Level::High);

        uart1.write(data).unwrap();

        led_write(&mut led, Level::Low);
        delay.delay(1000.millis());
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
