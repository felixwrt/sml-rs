//! SML transport protocol (version 1).
//!
//! *Hint: This crate currently only implements version 1 of the SML transport
//! protocol. If you need support for version 2, let me know!*
//!
//! # SML Transport Protocol - Version 1
//!
//! Version 1 of the SML Transport Protocol is a simple format that encodes binary messages using escape sequences. A message consists of the following parts (numbers in hex):
//!
//! - **Start sequence**: `1b1b1b1b 01010101`
//! - **Escaped data**: The data that should be encoded. If the escape sequence (`1b1b1b1b`) occurs in the data, it is escaped by an escape sequence (`1b1b1b1b`). For example, the data `001b1b1b 1b010203` would be encoded as `001b1b1b 1b1b1b1b 1b010203`.
//! - **Padding**: The data is zero-padded to the next multiple of four. Therefore, zero to three `0x00` bytes are inserted.
//! - **End sequence**: `1b1b1b1b 1aXXYYZZ`
//!   - `XX`: number of padding bytes
//!   - `YY`/`ZZ`: CRC checksum
//!
//! ## Encoding
//!
//! This crate implements both a streaming and a more traditional encoder.
//!
//! - `encode`: takes a sequence of bytes as input and returns a buffer containing the encoded message
//! - `encode_streaming`: an iterator adapter that encodes the input on the fly
//!
//!
//! ## Decoding
//!
//! - `decode`: takes a sequence of bytes and decodes them into a vector of messages / errors. Requires feature "alloc".
//! - `decode_streaming`: takes a sequence of bytes and returns an iterator over the decoded messages / errors.
//! - using `Decoder` directly: instantiate a `Decoder` manually, call `push_byte()` on it when data becomes available. Call `finalize()` when all data has been pushed.

mod decode;
mod decoder_reader;
mod encode;

#[cfg(feature = "alloc")]
pub use decode::decode;
pub use decode::{decode_streaming, DecodeErr, DecodeIterator, Decoder};
pub use decoder_reader::{DecoderReader, ReadDecodedError};
pub use encode::{encode, encode_streaming, Encoder};
