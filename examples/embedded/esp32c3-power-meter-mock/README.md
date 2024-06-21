# esp32c3 Power Meter Mock

This is a simple program for the ESP32-C3 that continuously sends sml messages, just like a digital german power meter.

The project is used to test the [`sml-rs`](https://github.com/felixwrt/sml-rs) library.

Pins:
- GPIO 9: Data is sent via this pin (TX).
- GPIO 8: This should be connected to the on-board RGB LED. It then indicates when data is being sent.

## Usage

Install [`espflash`](https://github.com/esp-rs/espflash/tree/main/espflash):

```
cargo install espflash
```

Flash and run the example:

```
cargo run --relase
```