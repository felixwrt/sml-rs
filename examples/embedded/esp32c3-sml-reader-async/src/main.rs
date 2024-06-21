#![no_std]
#![no_main]


use core::slice;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::Io,
    peripherals::{Peripherals, UART1},
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
    uart::{
        config::{Config, StopBits}, TxRxPins, Uart, UartRx
    },
    Async,
};
use sml_rs::{transport::Decoder, util::ArrayBuf};


#[embassy_executor::task]
async fn reader(
    mut rx: UartRx<'static, UART1, Async>,
) {
    esp_println::println!("Starting reader task!");
    let buf = ArrayBuf::<4069>::default();
    let mut decoder = Decoder::from_buf(buf);
    
    loop {
        let mut b = 0u8;
        let r = embedded_io_async::Read::read(&mut rx, slice::from_mut(&mut b)).await;
        match r {
            Ok(_) => {
                
                match decoder.push_byte(b) {
                    Ok(None) => {
                        continue;
                    }
                    Ok(Some(bytes)) => {
                        esp_println::println!("Got data: {bytes:?}")
                    }
                    Err(e) => {
                        esp_println::println!("Error receiving data: {e}")
                    }
                }
            }
            Err(e) => esp_println::println!("RX Error: {:?}", e),
        }
    }
}

#[embassy_executor::task]
async fn print() {
    loop {
        esp_println::println!("Hello world from the async SML Reader!");
        Timer::after(Duration::from_millis(2000)).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::println!("Init!");
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timg0);

    spawner.spawn(print()).ok();


    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // UART PORT
    let tx_pin = io.pins.gpio10;
    let rx_pin = io.pins.gpio9;
    let pins = TxRxPins::new_tx_rx(tx_pin, rx_pin);
    
    let uart_config = Config::default().baudrate(9600).parity_none().stop_bits(StopBits::STOP1);

    let mut uart1 = Uart::new_async_with_config(peripherals.UART1, uart_config, Some(pins), &clocks);
    uart1.set_rx_fifo_full_threshold(0).unwrap();
    
    let (_tx, rx) = uart1.split();

    spawner.spawn(reader(rx)).ok();
}