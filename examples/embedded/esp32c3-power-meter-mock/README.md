# esp32c3 Power Meter Mock

This is a simple program for the ESP32-C3 that continuously sends sml messages via UART.

The example is used to mock a digital german power meter and test the [`sml-rs`][1] library.

It sends an sml message approximately every second and turns an LED on while sending the data.

## Configuration

By default, the following pins are used:

- GPIO 9: UART TX pin used to send data
- GPIO 10: UART RX pin (unused, but needs to be provided to the UART peripheral)
- GPIO 18: LED pin (see below)

You can adapt the pin configuration in the source code (see the comment block "Pin configuration").

### Led configuration

This project can use either a smart RGB LED (such as the one found on the [ESP32-C3-DevKitC-02][2]) or a simple LED that can 
be driven by setting the output to high / low.

By default, this project assumes a regular LED. Activate the `smart-led` feature to use a smart RGB LED:

```
cargo ... --features smart-led
```

## Usage

Install [`espflash`][3]:

```
cargo install espflash
```

Flash and run the example:

```
cargo run --release
```

When using a smart RGB LED:

```
cargo run --release --features smart-led
```

[1]: https://github.com/felixwrt/sml-rs
[2]: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/hw-reference/esp32c3/user-guide-devkitc-02.html
[3]: https://github.com/esp-rs/espflash/tree/main/espflash