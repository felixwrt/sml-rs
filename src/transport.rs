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

mod decoder_reader;

pub use decoder_reader::{DecoderReader, ReadDecodedError};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use core::{borrow::Borrow, fmt};

use crate::util::{Buffer, OutOfMemory, CRC_X25};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

struct Padding(u8);

impl Padding {
    const fn new() -> Self {
        Padding(0)
    }

    fn bump(&mut self) {
        self.0 = self.0.wrapping_sub(1);
    }

    const fn get(&self) -> u8 {
        self.0 & 0x3
    }
}

#[derive(Debug, Clone, Copy)]
enum EncoderState {
    Init(u8),
    LookingForEscape(u8),
    HandlingEscape(u8),
    End(i8),
}

/// An iterator that encodes the bytes of an underlying iterator using the SML Transport Protocol v1.
pub struct Encoder<I>
where
    I: Iterator<Item = u8>,
{
    state: EncoderState,
    crc: crc::Digest<'static, u16>,
    padding: Padding,
    iter: I,
}

impl<I> Encoder<I>
where
    I: Iterator<Item = u8>,
{
    /// Creates an `Encoder` from a byte iterator.
    pub fn new(iter: I) -> Self {
        let mut crc = CRC_X25.digest();
        crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
        Encoder {
            state: EncoderState::Init(0),
            crc,
            padding: Padding::new(),
            iter,
        }
    }

    fn read_from_iter(&mut self) -> Option<u8> {
        let ret = self.iter.next();
        if ret.is_some() {
            self.padding.bump();
        }
        ret
    }

    fn next_from_state(&mut self, state: EncoderState) -> (Option<u8>, EncoderState) {
        self.state = state;
        let out = self.next();
        (out, self.state)
    }
}

impl<I> Iterator for Encoder<I>
where
    I: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        use EncoderState::*;
        let (out, state) = match self.state {
            Init(n) if n < 4 => (Some(0x1b), Init(n + 1)),
            Init(n) if n < 8 => (Some(0x01), Init(n + 1)),
            Init(n) => {
                assert_eq!(n, 8);
                self.next_from_state(LookingForEscape(0))
            }
            LookingForEscape(n) if n < 4 => {
                match self.read_from_iter() {
                    Some(b) => {
                        self.crc.update(&[b]);
                        (Some(b), LookingForEscape((n + 1) * u8::from(b == 0x1b)))
                    }
                    None => {
                        let padding = self.padding.get();
                        // finalize crc
                        for _ in 0..padding {
                            self.crc.update(&[0x00]);
                        }
                        self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x1a, padding]);
                        self.next_from_state(End(-(padding as i8)))
                    }
                }
            }
            LookingForEscape(n) => {
                assert_eq!(n, 4);
                self.crc.update(&[0x1b; 4]);
                self.next_from_state(HandlingEscape(0))
            }
            HandlingEscape(n) if n < 4 => (Some(0x1b), HandlingEscape(n + 1)),
            HandlingEscape(n) => {
                assert_eq!(n, 4);
                self.next_from_state(LookingForEscape(0))
            }
            End(n) => {
                let out = match n {
                    n if n < 0 => 0x00,
                    n if n < 4 => 0x1b,
                    4 => 0x1a,
                    5 => self.padding.get(),
                    n if n < 8 => {
                        let crc_bytes = self.crc.clone().finalize().to_le_bytes();
                        crc_bytes[(n - 6) as usize]
                    }
                    8 => {
                        return None;
                    }
                    _ => unreachable!(),
                };
                (Some(out), End(n + 1))
            }
        };
        self.state = state;
        out
    }
}

/// Takes a slice of bytes as input and returns a buffer containing the encoded message.
///
/// Returns `Err(())` when the buffer can't be grown to hold the entire output.
///
/// # Examples
///
/// ```
/// // example data
/// let bytes = [0x12, 0x34, 0x56, 0x78];
/// let expected = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// ```
///
/// ### Using alloc::Vec
///
/// ```
/// # #[cfg(feature = "alloc")] {
/// # use sml_rs::transport::encode;
/// # let bytes = [0x12, 0x34, 0x56, 0x78];
/// # let expected = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// let encoded = encode::<Vec<u8>>(&bytes);
/// assert!(encoded.is_ok());
/// assert_eq!(encoded.unwrap().as_slice(), &expected);
/// # }
/// ```
///
/// ### Using `ArrayBuf`
///
/// ```
/// # use sml_rs::{util::{ArrayBuf, OutOfMemory}, transport::encode};
/// # let bytes = [0x12, 0x34, 0x56, 0x78];
/// # let expected = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// let encoded = encode::<ArrayBuf<20>>(&bytes);
/// assert!(encoded.is_ok());
/// assert_eq!(&*encoded.unwrap(), &expected);
///
/// // encoding returns `Err(())` if the encoded message does not fit into the vector
/// let encoded = encode::<ArrayBuf<19>>(&bytes);
/// assert_eq!(encoded, Err(OutOfMemory));
/// ```
///
pub fn encode<B: Buffer>(
    iter: impl IntoIterator<Item = impl Borrow<u8>>,
) -> Result<B, OutOfMemory> {
    let mut res: B = Default::default();

    // start escape sequence
    res.extend_from_slice(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01])?;

    // encode data
    let mut num_1b = 0;
    for b in iter.into_iter() {
        let b = *b.borrow();
        if b == 0x1b {
            num_1b += 1;
        } else {
            num_1b = 0;
        }

        res.push(b)?;

        if num_1b == 4 {
            res.extend_from_slice(&[0x1b; 4])?;
            num_1b = 0;
        }
    }

    // padding bytes
    let num_padding_bytes = (4 - (res.len() % 4)) % 4;
    res.extend_from_slice(&[0x0; 3][..num_padding_bytes])?;

    res.extend_from_slice(&[0x1b, 0x1b, 0x1b, 0x1b, 0x1a, num_padding_bytes as u8])?;
    let crc = CRC_X25.checksum(&res[..]);

    res.extend_from_slice(&crc.to_le_bytes())?;

    Ok(res)
}

/// Takes an iterator over bytes and returns an iterator that produces the encoded message.
///
/// # Examples
/// ```
/// # use sml_rs::transport::encode_streaming;
/// // example data
/// let bytes = [0x12, 0x34, 0x56, 0x78];
/// let expected = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// let iter = encode_streaming(bytes);
/// assert!(iter.eq(expected));
/// ```
pub fn encode_streaming(
    iter: impl IntoIterator<Item = impl Borrow<u8>>,
) -> Encoder<impl Iterator<Item = u8>> {
    Encoder::new(iter.into_iter().map(|x| *x.borrow()))
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
/// An error which can be returned when decoding an sml message.
pub enum DecodeErr {
    /// Some bytes could not be parsed and were discarded
    DiscardedBytes(usize),
    /// An invalid escape sequence has been read
    InvalidEsc([u8; 4]),
    /// The buffer used internally by the encoder is full. When using vec, allocation has failed.
    OutOfMemory,
    /// The decoded message is invalid.
    InvalidMessage {
        /// (expected, found) checksums
        checksum_mismatch: (u16, u16),
        /// whether the end escape sequence wasn't aligned to a 4-byte boundary
        end_esc_misaligned: bool,
        /// the number of padding bytes.
        num_padding_bytes: u8,
    },
}

impl fmt::Display for DecodeErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeErr {}

#[derive(Debug)]
enum DecodeState {
    LookingForMessageStart {
        num_discarded_bytes: u16,
        num_init_seq_bytes: u8,
    },
    ParsingNormal,
    ParsingEscChars(u8),
    ParsingEscPayload(u8),
    Done,
}

/// Decoder for sml transport v1.
///
/// # Examples
///
/// ```
/// # use sml_rs::{util::ArrayBuf, transport::Decoder};
/// let bytes = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// let expected = [0x12, 0x34, 0x56, 0x78];
///
/// let mut decoder = Decoder::<ArrayBuf<20>>::new();
/// for b in bytes {
///     match decoder.push_byte(b) {
///         Ok(None) => {},  // nothing to output currently
///         Ok(Some(decoded)) => {  // complete and valid message was decoded
///             assert_eq!(decoded, expected);
///         }
///         Err(e) => {
///             panic!("Unexpected Error: {:?}", e);
///         }
///     }
/// }
/// assert_eq!(decoder.finalize(), None)
/// ```
pub struct Decoder<B: Buffer> {
    buf: B,
    raw_msg_len: usize,
    crc: crc::Digest<'static, u16>,
    crc_idx: usize,
    state: DecodeState,
}

impl<B: Buffer> Default for Decoder<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Buffer> Decoder<B> {
    /// Constructs a new decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::from_buf(Default::default())
    }

    /// Constructs a new decoder using an existing buffer `buf`.
    pub fn from_buf(mut buf: B) -> Self {
        buf.clear();
        Decoder {
            buf,
            raw_msg_len: 0,
            crc: CRC_X25.digest(),
            crc_idx: 0,
            state: DecodeState::LookingForMessageStart {
                num_discarded_bytes: 0,
                num_init_seq_bytes: 0,
            },
        }
    }

    /// Pushes a byte `b` into the decoder, advances the parser state and possibly returns
    /// a transmission or an decoder error.
    pub fn push_byte(&mut self, b: u8) -> Result<Option<&[u8]>, DecodeErr> {
        self._push_byte(b)
            .map(|b| if b { Some(self.borrow_buf()) } else { None })
    }

    /// Resets the `Decoder` and returns an error if it contained an incomplete message.
    pub fn finalize(&mut self) -> Option<DecodeErr> {
        use DecodeState::*;
        let res = match self.state {
            LookingForMessageStart {
                num_discarded_bytes: 0,
                num_init_seq_bytes: 0,
            } => None,
            Done => None,
            _ => Some(DecodeErr::DiscardedBytes(self.raw_msg_len)),
        };
        self.reset();
        res
    }

    /// Main function of the parser.
    ///
    /// Returns
    /// - `Ok(true)` if a complete message is ready.
    /// - `Ok(false)` when more bytes are necessary to complete parsing a message.
    /// - `Err(_)` if an error occurred during parsing
    pub(crate) fn _push_byte(&mut self, b: u8) -> Result<bool, DecodeErr> {
        use DecodeState::*;
        self.raw_msg_len += 1;
        match self.state {
            LookingForMessageStart {
                ref mut num_discarded_bytes,
                ref mut num_init_seq_bytes,
            } => {
                if (b == 0x1b && *num_init_seq_bytes < 4) || (b == 0x01 && *num_init_seq_bytes >= 4)
                {
                    *num_init_seq_bytes += 1;
                } else {
                    *num_discarded_bytes += 1 + u16::from(*num_init_seq_bytes);
                    *num_init_seq_bytes = 0;
                }
                if *num_init_seq_bytes == 8 {
                    let num_discarded_bytes = *num_discarded_bytes;
                    self.state = ParsingNormal;
                    self.raw_msg_len = 8;
                    assert_eq!(self.buf.len(), 0);
                    assert_eq!(self.crc_idx, 0);
                    self.crc = CRC_X25.digest();
                    self.crc
                        .update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
                    if num_discarded_bytes > 0 {
                        return Err(DecodeErr::DiscardedBytes(num_discarded_bytes as usize));
                    }
                }
            }
            ParsingNormal => {
                if b == 0x1b {
                    // this could be the first byte of an escape sequence
                    self.state = ParsingEscChars(1);
                } else {
                    // regular data
                    self.push(b)?;
                }
            }
            ParsingEscChars(n) => {
                if b != 0x1b {
                    // push previous 0x1b bytes as they didn't belong to an escape sequence
                    for _ in 0..n {
                        self.push(0x1b)?;
                    }
                    // push current byte
                    self.push(b)?;
                    // continue in regular parsing state
                    self.state = ParsingNormal;
                } else if n == 3 {
                    // this is the fourth 0x1b byte, so we're seeing an escape sequence.
                    // continue by parsing the escape sequence's payload.

                    // also update the crc here. the escape bytes aren't stored in `buf`, but
                    // still need to count for the crc calculation
                    // (1) add everything that's in the buffer and hasn't been added to the crc previously
                    self.crc.update(&self.buf[self.crc_idx..self.buf.len()]);
                    // (2) add the four escape bytes
                    self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b]);
                    // update crc_idx to indicate that everything that's currently in the buffer has already
                    // been used to update the crc
                    self.crc_idx = self.buf.len();

                    self.state = ParsingEscPayload(0);
                } else {
                    self.state = ParsingEscChars(n + 1);
                }
            }
            ParsingEscPayload(n) => {
                self.push(b)?;
                if n < 3 {
                    self.state = ParsingEscPayload(n + 1);
                } else {
                    // last 4 elements in self.buf are the escape sequence payload
                    let payload = &self.buf[self.buf.len() - 4..self.buf.len()];
                    if payload == [0x1b, 0x1b, 0x1b, 0x1b] {
                        // escape sequence in user data

                        // nothing to do here as the input has already been added to the buffer (see above)
                        self.state = ParsingNormal;
                    } else if payload == [0x01, 0x01, 0x01, 0x01] {
                        // another transmission start

                        // ignore everything that has previously been read and start reading a new transmission
                        let ignored_bytes = self.raw_msg_len - 8;
                        self.raw_msg_len = 8;
                        self.buf.clear();
                        self.crc = CRC_X25.digest();
                        self.crc
                            .update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
                        self.crc_idx = 0;
                        self.state = ParsingNormal;
                        return Err(DecodeErr::DiscardedBytes(ignored_bytes));
                    } else if payload[0] == 0x1a {
                        // end sequence (layout: [0x1a, num_padding_bytes, crc, crc])

                        // check number of padding bytes
                        let num_padding_bytes = payload[1];

                        // compute and compare checksum
                        let read_crc = u16::from_le_bytes([payload[2], payload[3]]);
                        // update the crc, but exclude the last two bytes (which contain the crc itself)
                        self.crc
                            .update(&self.buf[self.crc_idx..(self.buf.len() - 2)]);
                        // get the calculated crc and reset it afterwards
                        let calculated_crc = {
                            let mut crc = CRC_X25.digest();
                            core::mem::swap(&mut crc, &mut self.crc);
                            crc.finalize()
                        };

                        // check alignment (end marker needs to have 4-byte alignment)
                        let misaligned = self.buf.len() % 4 != 0;

                        // check if padding is larger than the message length
                        let padding_too_large = num_padding_bytes > 3
                            || (num_padding_bytes as usize + 4) > self.buf.len();

                        if read_crc != calculated_crc || misaligned || padding_too_large {
                            self.reset();
                            return Err(DecodeErr::InvalidMessage {
                                checksum_mismatch: (read_crc, calculated_crc),
                                end_esc_misaligned: misaligned,
                                num_padding_bytes,
                            });
                        }

                        // subtract padding bytes and escape payload length from buffer length
                        self.buf
                            .truncate(self.buf.len() - num_padding_bytes as usize - 4);

                        self.set_done();

                        return Ok(true);
                    } else {
                        // special case of message ending with incomplete escape sequence
                        // Explanation:
                        // when a message ends with 1-3 0x1b bytes and there's no padding bytes,
                        // we end up in this branch because there's four consecutive 0x1b bytes
                        // that aren't followed by a known escape sequence. The problem is that
                        // the first 1-3 0x1b bytes belong to the message, not to the end escape
                        // code.
                        // Example:
                        //                  detected as escape sequence
                        //                  vvvv vvvv
                        // Message: ... 12341b1b 1b1b1b1b 1a00abcd
                        //                       ^^^^^^^^
                        //                       real escape sequence
                        //
                        // The solution for this issue is to check whether the read esacpe code
                        // isn't aligned to a 4-byte boundary and followed by an aligned end
                        // escape sequence (`1b1b1b1b 1a...`).
                        // If that's the case, simply reset the parser state by 1-3 steps. This
                        // will parse the 0x1b bytes in the message as regular bytes and check
                        // for the end escape code at the right position.
                        let bytes_until_alignment = (4 - (self.buf.len() % 4)) % 4;
                        if bytes_until_alignment > 0
                            && payload[..bytes_until_alignment].iter().all(|x| *x == 0x1b)
                            && payload[bytes_until_alignment] == 0x1a
                        {
                            self.state = ParsingEscPayload(4 - bytes_until_alignment as u8);
                            return Ok(false);
                        }

                        // invalid escape sequence

                        // unwrap is safe here because payload is guaranteed to have size 4
                        let esc_bytes: [u8; 4] = payload.try_into().unwrap();
                        self.reset();
                        return Err(DecodeErr::InvalidEsc(esc_bytes));
                    }
                }
            }
            Done => {
                // reset and let's go again
                self.reset();
                return self._push_byte(b);
            }
        }
        Ok(false)
    }

    pub(crate) fn borrow_buf(&self) -> &[u8] {
        if !matches!(self.state, DecodeState::Done) {
            panic!("Reading from the internal buffer is only allowed when a complete message is present (DecodeState::Done). Found state {:?}.", self.state);
        }
        &self.buf[..self.buf.len()]
    }

    fn set_done(&mut self) {
        self.state = DecodeState::Done;
    }

    /// Resets the `Decoder` and returns the number of bytes that were discarded
    pub fn reset(&mut self) -> usize {
        let num_discarded = match self.state {
            DecodeState::Done => 0,
            _ => self.raw_msg_len,
        };
        self.state = DecodeState::LookingForMessageStart {
            num_discarded_bytes: 0,
            num_init_seq_bytes: 0,
        };
        self.buf.clear();
        self.crc_idx = 0;
        self.raw_msg_len = 0;
        num_discarded
    }

    fn push(&mut self, b: u8) -> Result<(), DecodeErr> {
        if self.buf.push(b).is_err() {
            self.reset();
            return Err(DecodeErr::OutOfMemory);
        }
        Ok(())
    }
}

/// Decode a given slice of bytes and returns a vector of messages / errors.
///
/// *This function is available only if sml-rs is built with the `"alloc"` feature.*
///
/// # Examples
/// ```
/// # use sml_rs::transport::decode;
/// // example data
/// let bytes = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
/// let expected = [0x12, 0x34, 0x56, 0x78];
/// let decoded = decode(&bytes);
/// assert_eq!(decoded, vec!(Ok(expected.to_vec())));
#[cfg(feature = "alloc")]
#[must_use]
pub fn decode(iter: impl IntoIterator<Item = impl Borrow<u8>>) -> Vec<Result<Vec<u8>, DecodeErr>> {
    let mut decoder: Decoder<Vec<u8>> = Decoder::new();
    let mut res = Vec::new();
    for b in iter.into_iter() {
        match decoder.push_byte(*b.borrow()) {
            Ok(None) => {}
            Ok(Some(buf)) => res.push(Ok(buf.to_vec())),
            Err(e) => res.push(Err(e)),
        }
    }
    if let Some(e) = decoder.finalize() {
        res.push(Err(e));
    }
    res
}

/// Iterator over decoded messages / errors.
pub struct DecodeIterator<B: Buffer, I: Iterator<Item = u8>> {
    decoder: Decoder<B>,
    bytes: I,
    done: bool,
}

impl<B: Buffer, I: Iterator<Item = u8>> DecodeIterator<B, I> {
    fn new(bytes: I) -> Self {
        DecodeIterator {
            decoder: Decoder::new(),
            bytes,
            done: false,
        }
    }

    /// Returns the next message / error.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<&[u8], DecodeErr>> {
        if self.done {
            return None;
        }
        loop {
            match self.bytes.next() {
                Some(b) => {
                    match self.decoder._push_byte(b) {
                        Ok(true) => return Some(Ok(self.decoder.borrow_buf())),
                        Err(e) => {
                            return Some(Err(e));
                        }
                        Ok(false) => {
                            // take next byte...
                        }
                    }
                }
                None => {
                    self.done = true;
                    return self.decoder.finalize().map(Err);
                }
            }
        }
    }
}

/// Takes an iterator over bytes and returns an iterator that yields decoded messages / decoding errors.
///
/// # Examples
/// ```
/// # use sml_rs::{util::ArrayBuf, transport::decode_streaming};
/// // example data
/// let bytes = [
///     // first message
///     0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b,
///     // second message
///     0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x13, 0x24, 0x35, 0x46, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb1, 0xa1,
/// ];
/// let mut decode_iterator = decode_streaming::<ArrayBuf<10>>(&bytes);
/// assert_eq!(decode_iterator.next(), Some(Ok([0x12, 0x34, 0x56, 0x78].as_slice())));
/// assert_eq!(decode_iterator.next(), Some(Ok([0x13, 0x24, 0x35, 0x46].as_slice())));
/// assert_eq!(decode_iterator.next(), None);
pub fn decode_streaming<B: Buffer>(
    iter: impl IntoIterator<Item = impl Borrow<u8>>,
) -> DecodeIterator<B, impl Iterator<Item = u8>> {
    DecodeIterator::new(iter.into_iter().map(|x| *x.borrow()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // assert_eq macro that prints its arguments as hex when they don't match.
    // (adapted from the `assert_hex` crate)
    macro_rules! assert_eq_hex {
        ($left:expr, $right:expr $(,)?) => {{
            match (&$left, &$right) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        // The reborrows below are intentional. Without them, the stack slot for the
                        // borrow is initialized even before the values are compared, leading to a
                        // noticeable slow down.
                        panic!(
                            "assertion failed: `(left == right)`\n  left: `{:02x?}`,\n right: `{:02x?}`",
                            &*left_val, &*right_val
                        )
                    }
                }
            }
        }};
    }

    fn test_encoding<const N: usize>(bytes: &[u8], exp_encoded_bytes: &[u8; N]) {
        compare_encoded_bytes(
            exp_encoded_bytes,
            &encode::<crate::util::ArrayBuf<N>>(bytes).expect("ran out of memory"),
        );
        compare_encoded_bytes(
            exp_encoded_bytes,
            &encode_streaming(bytes).collect::<crate::util::ArrayBuf<N>>(),
        );
        #[cfg(feature = "alloc")]
        assert_eq_hex!(alloc::vec![Ok(bytes.to_vec())], decode(exp_encoded_bytes));
    }

    fn compare_encoded_bytes(expected: &[u8], actual: &[u8]) {
        assert_eq_hex!(expected, actual);
    }

    #[test]
    fn basic() {
        test_encoding(
            &hex!("12345678"),
            &hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b"),
        );
    }

    #[test]
    fn empty() {
        test_encoding(&hex!(""), &hex!("1b1b1b1b 01010101 1b1b1b1b 1a00c6e5"));
    }

    #[test]
    fn padding() {
        test_encoding(
            &hex!("123456"),
            &hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5"),
        );
    }

    #[test]
    fn escape_in_user_data() {
        test_encoding(
            &hex!("121b1b1b1b"),
            &hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25"),
        );
    }

    #[test]
    fn almost_escape_in_user_data() {
        test_encoding(
            &hex!("121b1b1bFF"),
            &hex!("1b1b1b1b 01010101 12 1b1b1bFF 000000 1b1b1b1b 1a0324d9"),
        );
    }

    #[test]
    fn ending_with_1b_no_padding() {
        test_encoding(
            &hex!("12345678 12341b1b"),
            &hex!("1b1b1b1b 01010101 12345678 12341b1b 1b1b1b1b 1a001ac5"),
        );
    }
}

#[cfg(test)]
mod decode_tests {
    use super::*;
    use crate::util::ArrayBuf;
    use hex_literal::hex;
    use DecodeErr::*;

    fn test_parse_input<B: Buffer>(bytes: &[u8], exp: &[Result<&[u8], DecodeErr>]) {
        // check that the streaming decoder yields the expected data
        let mut exp_iter = exp.iter();
        let mut streaming_decoder = DecodeIterator::<B, _>::new(bytes.iter().cloned());

        while let Some(res) = streaming_decoder.next() {
            match exp_iter.next() {
                Some(exp) => {
                    assert_eq!(res, *exp);
                }
                None => {
                    panic!("Additional decoded item: {:?}", res);
                }
            }
        }
        assert_eq!(exp_iter.next(), None);

        // check that Decoder and DecodeIterator yield the same data:
        let mut decoder = Decoder::<B>::new();
        let mut streaming_decoder = DecodeIterator::<B, _>::new(bytes.iter().cloned());
        for b in bytes {
            let res = decoder.push_byte(*b);
            if let Ok(None) = res {
                continue;
            }
            let res2 = streaming_decoder.next();
            match (res, res2) {
                (Ok(Some(a)), Some(Ok(b))) => assert_eq!(a, b),
                (Err(a), Some(Err(b))) => assert_eq!(a, b),
                (a, b) => panic!(
                    "Mismatch between decoder and streaming_decoder: {:?} vs. {:?}",
                    a, b
                ),
            }
        }
        match (decoder.finalize(), streaming_decoder.next()) {
            (None, None) => {},
            (Some(a), Some(Err(b))) => assert_eq!(a, b),
            (a, b) => panic!("Mismatch between decoder and streaming_decoder on the final element: {:?} vs. {:?}", a, b),
        }
    }

    #[test]
    fn basic() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[Ok(hex!("12345678").as_slice())];

        test_parse_input::<ArrayBuf<8>>(&bytes, exp);
    }

    #[test]
    fn out_of_memory() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[Err(DecodeErr::OutOfMemory)];

        test_parse_input::<ArrayBuf<7>>(&bytes, exp);
    }

    #[test]
    fn invalid_crc() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b8FF");
        let exp = &[Err(InvalidMessage {
            checksum_mismatch: (0xFFb8, 0x7bb8),
            end_esc_misaligned: false,
            num_padding_bytes: 0,
        })];

        test_parse_input::<ArrayBuf<8>>(&bytes, exp);
    }

    #[test]
    fn msg_end_misaligned() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 FF 1b1b1b1b 1a0013b6");
        let exp = &[Err(InvalidMessage {
            checksum_mismatch: (0xb613, 0xb613),
            end_esc_misaligned: true,
            num_padding_bytes: 0,
        })];

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
    }

    #[test]
    fn padding_too_large() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 12345678 1b1b1b1b 1a04f950");
        let exp = &[Err(InvalidMessage {
            checksum_mismatch: (0x50f9, 0x50f9),
            end_esc_misaligned: false,
            num_padding_bytes: 4,
        })];

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
    }

    #[test]
    fn empty_msg_with_padding() {
        let bytes = hex!("1b1b1b1b 01010101 1b1b1b1b 1a014FF4");
        let exp = &[Err(InvalidMessage {
            checksum_mismatch: (0xf44f, 0xf44f),
            end_esc_misaligned: false,
            num_padding_bytes: 1,
        })];

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
    }

    #[test]
    fn additional_bytes() {
        let bytes = hex!("000102 1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b 1234");
        let exp = &[
            Err(DiscardedBytes(3)),
            Ok(hex!("12345678").as_slice()),
            Err(DiscardedBytes(2)),
        ];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn incomplete_message() {
        let bytes = hex!("1b1b1b1b 01010101 123456");
        let exp = &[Err(DiscardedBytes(11))];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn invalid_esc_sequence() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1c000000 12345678 1b1b1b1b 1a03be25");
        let exp = &[
            Err(InvalidEsc([0x1c, 0x0, 0x0, 0x0])),
            Err(DiscardedBytes(12)),
        ];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn incomplete_esc_sequence() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b00 12345678 1b1b1b1b 1a030A07");
        let exp = &[Ok(hex!("12345678 1b1b1b00 12").as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn double_msg_start() {
        let bytes =
            hex!("1b1b1b1b 01010101 09 87654321 1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[Err(DiscardedBytes(13)), Ok(hex!("12345678").as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn padding() {
        let bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");
        let exp_bytes = hex!("123456");
        let exp = &[Ok(exp_bytes.as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");
        let exp = &[Ok(hex!("121b1b1b1b").as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn ending_with_1b_no_padding_1() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1234561b 1b1b1b1b 1a00361a");
        let exp_bytes = hex!("12345678 1234561b");
        let exp = &[Ok(exp_bytes.as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn ending_with_1b_no_padding_2() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 12341b1b 1b1b1b1b 1a001ac5");
        let exp_bytes = hex!("12345678 12341b1b");
        let exp = &[Ok(exp_bytes.as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn ending_with_1b_no_padding_3() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 121b1b1b 1b1b1b1b 1a000ba4");
        let exp_bytes = hex!("12345678 121b1b1b");
        let exp = &[Ok(exp_bytes.as_slice())];

        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn alloc_basic() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[Ok(hex!("12345678").as_slice())];

        test_parse_input::<Vec<u8>>(&bytes, exp);
    }
}
