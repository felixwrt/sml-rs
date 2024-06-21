# esp32c3 SML Reader

This is a simple program for the ESP32-C3 that reads sml messages from a pin.

The project shows how to use the [`sml-rs`](https://github.com/felixwrt/sml-rs) library in an embedded context.

Pins:
- GPIO 9: Data is read from this pin (RX).
- GPIO 18: LED indicating when data is being received.

## Usage

Install [`espflash`](https://github.com/esp-rs/espflash/tree/main/espflash):

```
cargo install espflash
```

Flash and run the example:

```
cargo run --relase
```