# esp32c3 Async SML Reader

This is a simple program for the ESP32-C3 that reads sml messages from a UART pin.

The example shows how to use the [`sml-rs`][1] library in an async embedded context.

It receives data from a UART pin, tries to parse it as an sml message and prints the result on the terminal.
Additionally, it toggles an LED whenever a byte is received on the UART pin.

Note: also take a look at the [`esp32c3-power-meter-mock`](../esp32c3-power-meter-mock/) example which can be used to generate 
sml messages that can be consumed by this example.

## Configuration

By default, the following pins are used:

- GPIO 10: UART TX pin (unused, but needs to be provided to the UART peripheral)
- GPIO 9: UART RX pin used to receive data
- GPIO 8: LED pin (see below)

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
cargo run --relase
```

When using a smart RGB LED:

```
cargo run --relase --features smart-led
```

[1]: https://github.com/felixwrt/sml-rs
[2]: https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/hw-reference/esp32c3/user-guide-devkitc-02.html
[3]: https://github.com/esp-rs/espflash/tree/main/espflash
