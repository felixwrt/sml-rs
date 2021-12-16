use anyhow::{Result, bail};

extern crate alloc;
use core::convert::TryInto;
// first part
// - read bytes and (possibly) produce esc seq, bytes or errors

// pub struct LexState {
//     num_escape_bytes: u8,
//     bytes: [u8; 4]
// }

// impl LexState {
//     pub fn new() -> Self {
//         LexState {
//             num_escape_bytes: 0,
//             bytes: [0; 4],
//         }
//     }

//     pub fn push_byte(&mut self, b: u8) -> LexData {
//         if self.num_escape_bytes < 4 {
//             if b == 0x1b {
//                 // next byte of a possible escape sequence has been read
//                 self.num_escape_bytes += 1;
//                 LexData::None
//             } else {
//                 // it's not an excape sequence
//                 let ret = LexData::ByteAndNumEscBytes(b, self.num_escape_bytes);
//                 self.num_escape_bytes = 0;
//                 ret
//             }
//         } else {
//             // escape sequence (4x 0x1b) has been read, now read four more bytes
//             self.bytes[self.num_escape_bytes as usize - 4] = b;
//             self.num_escape_bytes += 1;
//             if self.num_escape_bytes == 8 {
//                 LexData::Escaped(self.bytes.clone())
//             } else {
//                 LexData::None
//             }
//         }
//     }
// }

// pub enum LexData {
//     Escaped([u8; 4]),
//     ByteAndNumEscBytes(u8, u8),  // byte, num_previous_esc_bytes
//     None
// }

// pub enum LexData2 {
//     Start,
//     End,
//     InvalidEsc,

// }


// pub struct Parser<const N: usize> {
//     buf: [u8; N],
//     buf_len: usize,
//     is_active: bool,
// }

// impl<const N: usize> Parser<N> {
//     fn new() -> Self {
//         Parser { 
//             buf: [0; N], 
//             buf_len: 0, 
//             is_active: false,
//         }
//     }

//     fn push_lex_data(&mut self, d: LexData) -> Result<Option<&[u8]>> {
//         match d {
//             LexData::None => Ok(None),
//             LexData::ByteAndNumEscBytes(b, num_esc_bytes) => {
//                 if self.is_active { 
//                     for _ in 0..num_esc_bytes {
//                         self.push(0x1b)?;
//                     }
//                     self.push(b)?;
//                 }
//                 Ok(None)
//             }
//             LexData::Escaped(bytes) => {
//                 if bytes == [0x01; 4] {
//                     // start sequence
//                     if !self.is_active {
//                         self.is_active = true;
//                     } else {
//                         // two start sequences. reset state and output warning
//                         self.buf_len = 0;
//                         // TODO: output warning
//                     }
//                     Ok(None)
//                 } else if bytes == [0x1b; 4]  {
//                     // escape sequence in user data
//                     for _ in 0..4 {
//                         self.push(0x1b);
//                     }
//                 } else if bytes[0] == 0x1a {
//                     // end sequence
//                     if !self.is_active {
//                         // TODO: should be a warning
//                         bail!("End without a start")
//                     }

//                 }
//                 unimplemented!()
//             }
//         }
//     }

//     fn push(&mut self, b: u8) -> Result<()> {
//         if self.buf_len >= N {
//             bail!("Buffer overflow")
//         }
//         self.buf[self.buf_len] = b;
//         self.buf_len += 1;
//         Ok(())
//     }
// }


// States:
// - Uninitialized: Start sequence hasn't been read
//   - bytes are discarded

static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

pub struct PowerMeterReader<Rx, const N: usize> 
where
    Rx: Iterator<Item=u8>
{
    rx: Rx,
    buf: [u8; N],
    buf_len: usize,
    is_active: bool,
    state: ReaderState
}


enum ReaderState {
    LookForEsc(u8),
    ReadEsc(u8),
}

impl<Rx, const N: usize> PowerMeterReader<Rx, N>
where 
    Rx: Iterator<Item=u8>
{
    pub fn new(rx: Rx) -> Self {
        PowerMeterReader {
            rx: rx,
            is_active: false,
            buf: [0; N],
            buf_len: 0,
            state: ReaderState::LookForEsc(0)
        }
    }

    // Reads bytes from the iterator
    // discards bytes until start escape code was found
    // returns buffer when end escape code was found
    // buffer contains escape codes just like they appear in the input
    #[inline(never)]
    pub fn read_message(&mut self) -> Result<([u8;N], usize)> {
        loop {
            match self.rx.next() {
                Some(b) => {
                    self.push(b)?;
                    self.state = match self.state {
                        ReaderState::LookForEsc(n) => {
                            if b == 0x1b && n==3 {
                                ReaderState::ReadEsc(0)
                            } else if b == 0x1b {
                                ReaderState::LookForEsc(n+1)
                            } else {
                                if !self.is_active {
                                    self.clear();
                                }
                                ReaderState::LookForEsc(0)
                            }
                        }
                        ReaderState::ReadEsc(n) => {
                            if n == 3 {
                                // complete escape sequence read
                                let esc_bytes = &self.buf[self.buf_len-4..self.buf_len];
                                if esc_bytes[0] == 0x1a {
                                    // end of transmission
                                    if !self.is_active {
                                        self.clear();
                                    } else {
                                        self.is_active = false;
                                        let msg = self.buf.clone();
                                        self.state = ReaderState::LookForEsc(0);
                                        let ret =  Ok((msg, self.buf_len));
                                        self.buf_len = 0;
                                        return ret;
                                    }
                                } else if esc_bytes == &[0x01; 4] {
                                    // start sequence
                                    // TODO: handle case that we're already active (so two activates appearing in the stream)
                                    self.is_active = true;
                                } else if esc_bytes == &[0x1b; 4] {
                                    // escape sequence in user data
                                } else if !self.is_active {
                                    // TODO: shouldn't this case also be valid without the "!self.is_active" condition?
                                    bail!("Found invalid escape sequence");
                                }
                                ReaderState::LookForEsc(0)
                            } else {
                                ReaderState::ReadEsc(n+1)
                            }
                        }
                    };
                }
                None => {
                    bail!("end of input")
                }
            }
        }
    }

    fn push(&mut self, b: u8) -> Result<()> {
        if self.buf_len >= N {
            bail!("Buffer overflow")
        }
        self.buf[self.buf_len] = b;
        self.buf_len += 1;
        Ok(())
    }

    fn clear(&mut self) {
        self.buf_len = 0;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseRes<'a> {
    // None, // just read a byte, nothing to report yet
    DiscardedBytes(usize), // just found the start of a transmission, but some previous bytes could not be parsed
    Transmission(&'a [u8]), // a full & valid transmission has been read. These are the bytes that make the message
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseErr {
    InvalidEsc([u8; 4]), // an invalid escape sequence has been read
    OutOfMemory, // the buffer used internally is full. When using vec, allocation has failed
    InvalidMessage {
        checksum_mismatch: (u16, u16),  // (expected, found)
        end_esc_misaligned: bool,
        num_padding_bytes: u8,
    }
}


pub struct SmlReader2<const N: usize> {
    buf: heapless::Vec<u8, N>,
    // buf: [u8; N],
    // buf_len: usize,
    raw_msg_len: usize,
    crc: crc::Digest<'static, u16>,
    crc_idx: usize,
    state: ParseState
}

impl<const N: usize> SmlReader2<N> {
    pub fn new() -> Self {
        SmlReader2 {
            buf: heapless::Vec::new(),
            // buf: [0; N],
            // buf_len: 0,
            raw_msg_len: 0,
            crc: CRC_X25.digest(),
            crc_idx: 0,
            state: ParseState::LookingForMessageStart {
                num_discarded_bytes: 0,
                num_init_seq_bytes: 0,
            }
        }
    }

    pub fn push_byte(&mut self, b: u8) -> Result<Option<ParseRes>, ParseErr> {
        self.raw_msg_len += 1;
        match self.state {
            ParseState::LookingForMessageStart {
                ref mut num_discarded_bytes, ref mut num_init_seq_bytes
            } => {
                if (b == 0x1b && *num_init_seq_bytes < 4) || (b == 0x01 && *num_init_seq_bytes >= 4) {
                    *num_init_seq_bytes += 1;
                } else {
                    *num_discarded_bytes += 1 + *num_init_seq_bytes as u16;
                    *num_init_seq_bytes = 0;
                }
                if *num_init_seq_bytes == 8 {
                    let num_discarded_bytes = *num_discarded_bytes;
                    self.state = ParseState::ParsingNormal;
                    assert_eq!(self.buf.len(), 0);
                    assert_eq!(self.crc_idx, 0);
                    self.crc = CRC_X25.digest();
                    self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
                    if num_discarded_bytes > 0 {
                        return Ok(Some(ParseRes::DiscardedBytes(num_discarded_bytes as usize)))
                    }
                }
            },
            ParseState::ParsingNormal => { 
                if b == 0x1b {
                    // this could be the first byte of an escape sequence
                    self.state = ParseState::ParsingEscChars(1);
                } else {
                    // regular data
                    self.push(b)?;
                }
            },
            ParseState::ParsingEscChars(n) => {
                if b != 0x1b {
                    // push previous 0x1b bytes as they didn't belong to an escape sequence
                    for _ in 0..n {
                        self.push(0x1b)?;
                    }
                    // push current byte
                    self.push(b)?;
                    // continue in regular parsing state
                    self.state = ParseState::ParsingNormal;
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

                    self.state = ParseState::ParsingEscPayload(0);
                } else {
                    self.state = ParseState::ParsingEscChars(n+1);
                }
            },
            ParseState::ParsingEscPayload(n) => {
                self.push(b)?;
                if n < 3 {
                    self.state = ParseState::ParsingEscPayload(n+1);
                } else {
                    // last 4 elements in self.buf are the escape sequence payload
                    let payload = &self.buf[self.buf.len()-4..self.buf.len()];
                    if payload == &[0x1b, 0x1b, 0x1b, 0x1b] {
                        // escape sequence in user data
                        
                        // nothing to do here as the input has already been added to the buffer (see above)
                        self.state = ParseState::ParsingNormal;
                    } else if payload[0] == 0x1a {
                        // end sequence (layout: [0x1a, num_padding_bytes, crc, crc])
                        
                        // check number of padding bytes
                        let num_padding_bytes = payload[1];

                        // compute and compare checksum
                        let read_crc = u16::from_le_bytes([payload[2], payload[3]]);
                        // update the crc, but exclude the last two bytes (which contain the crc itself)
                        self.crc.update(&self.buf[self.crc_idx..(self.buf.len()-2)]);
                        // get the calculated crc and reset it afterwards
                        let calculated_crc = {
                            let mut crc = CRC_X25.digest();
                            core::mem::swap(&mut crc, &mut self.crc);
                            crc.finalize()
                        };
                        
                        // check alignment (end marker needs to have 4-byte alignment)
                        let misaligned = self.buf.len() % 4 != 0;

                        // check if padding is larger than the message length
                        let padding_too_large = num_padding_bytes > 3 || (num_padding_bytes as usize + 4) > self.buf.len();

                        if read_crc != calculated_crc || misaligned || padding_too_large {
                            self.set_done();
                            return Err(ParseErr::InvalidMessage {
                                checksum_mismatch: (read_crc, calculated_crc),
                                end_esc_misaligned: misaligned,
                                num_padding_bytes: num_padding_bytes,
                            });
                        }

                        // subtract padding bytes and escape payload length from buffer length
                        self.buf.truncate(self.buf.len() - num_padding_bytes as usize - 4);

                        let len = self.buf.len();
                        self.set_done();

                        return Ok(Some(ParseRes::Transmission(&self.buf[..len])));
                    } else {
                        // invalid escape sequence
                        
                        // unwrap is safe here because payload is guaranteed to have size 4
                        let esc_bytes: [u8; 4] = payload.try_into().unwrap();
                        self.set_done();
                        return Err(ParseErr::InvalidEsc(esc_bytes));
                    }
                }
            }
            ParseState::Done => {
                // reset and let's go again
                self.reset();
                return self.push_byte(b);
            }
        }
        Ok(None)
    }

    pub fn finalize(self) -> Option<ParseRes<'static>> {
        match self.state {
            ParseState::LookingForMessageStart {
                num_discarded_bytes: 0, num_init_seq_bytes: 0
            } => {
                None
            },
            ParseState::LookingForMessageStart {
                num_discarded_bytes, num_init_seq_bytes
            } => {
                Some(ParseRes::DiscardedBytes(num_discarded_bytes as usize + num_init_seq_bytes as usize))
            }
            ParseState::Done => {
                None
            }
            _ => {
                Some(ParseRes::DiscardedBytes(self.raw_msg_len))
            }
        }
    }

    fn set_done(&mut self) {
        self.state = ParseState::Done;
    }

    fn reset(&mut self) {
        self.state = ParseState::LookingForMessageStart {
            num_discarded_bytes: 0,
            num_init_seq_bytes: 0
        };
        self.buf.clear();
        self.crc_idx = 0;
        self.raw_msg_len = 0;
    }

    fn push(&mut self, b: u8) -> Result<(), ParseErr> {
        if self.buf.push(b).is_err() {
            self.reset();
            return Err(ParseErr::OutOfMemory)
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ParseState {
    LookingForMessageStart {
        num_discarded_bytes: u16,
        num_init_seq_bytes: u8,
    },
    ParsingNormal,
    ParsingEscChars(u8),
    ParsingEscPayload(u8),
    Done,
}


pub struct SmlReader<Rx, const N: usize> 
where
    Rx: Iterator<Item=u8>
{
    rx: Rx,
    init: u8,
    num_discarded_bytes: usize,
    num_0x1b: u8,
    buf: [u8; N],
    buf_len: usize, 
    crc: crc::Digest<'static, u16>,
}

impl<Rx, const N: usize> SmlReader<Rx, N>
where
    Rx: Iterator<Item=u8>
{
    pub fn new(rx: Rx) -> Self {
        SmlReader {
            rx: rx,
            init: 0,
            num_discarded_bytes: 0,
            num_0x1b: 0,
            buf: [0; N],
            buf_len: 0,
            crc: CRC_X25.digest(),
        }
    }

    pub fn read_transmission_into_array(&mut self) -> Result<([u8; N], usize)> {
        let len = self.read_transmission_inner()?;
        Ok((self.buf, len))
    }

    pub fn read_transmission_into_slice(&mut self) -> Result<&[u8]> {
        let len = self.read_transmission_inner()?;
        Ok(&self.buf[..len])
    }

    // reads until start sequence is found
    // start sequence is not put into the buffer, but counts for crc
    // put bytes into buffer, user escapes count for crc, but aren't put into the buffer twice
    // when reading the end sequence, crc is compared and data buffer (excluding start/-end sequence and user escapes) is returned
    pub fn read_transmission_inner(&mut self) -> Result<usize> {
        while !self.initialized() {
            let b = self.read_byte()?;
            self.parse_init_seq(b);
        }

        if self.num_discarded_bytes > 0 {
            //format!("Discarded {} bytes", self.num_discarded_bytes);
        }
        
        // initialize crc with the start sequence that has already been read
        self.crc = CRC_X25.digest();
        self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);

        loop {
            let b = self.read_byte()?;
            if b == 0x1b {
                self.num_0x1b += 1;
                if self.num_0x1b == 4 {
                    // escape sequence found
                    let bytes = self.read_4_bytes()?;
                    
                    if bytes == [0x1b, 0x1b, 0x1b, 0x1b] {
                        // println!("esc user data");
                        // escape sequence in user data
                        for b in bytes {
                            self.push(b)?;
                        }
                        self.num_0x1b = 0;
                        self.crc.update(&bytes);
                    } else if bytes[0] == 0x1a {
                        if self.buf_len % 4 != 0 {
                            self.bail("end of transmission expected to have 4-byte alignment")?;
                        }
                        // end of transmission
                        let num_padding_bytes = bytes[1];
                        if num_padding_bytes > 3 {
                            self.bail("Invalid number of padding bytes")?;
                        }
                        self.crc.update(&bytes[..2]);
                        let checksum = u16::from_le_bytes([bytes[2], bytes[3]]);
                        
                        // get the calculated crc and reset it afterwards
                        let mut crc = CRC_X25.digest();
                        core::mem::swap(&mut crc, &mut self.crc);
                        let calculated_crc = crc.finalize();
                        
                        if calculated_crc != checksum {
                            self.bail("Checksum doesn't match")?;
                        }

                        self.buf_len -= num_padding_bytes as usize;
                        let len = self.buf_len;
                        self.reset();

                        return Ok(len);
                    } else {
                        bail!("Invalid escape sequence read")
                    }
                }
            } else {
                // no escape sequence, so add delayed values
                for _ in 0..self.num_0x1b {
                    self.push(0x1b)?;
                }
                self.num_0x1b = 0;
                self.push(b)?;
            }
        }
    }

    fn read_byte(&mut self) -> Result<u8> {
        let b = self.read_byte_no_crc()?;
        self.crc.update(&[b]);
        Ok(b)
    }

    fn read_byte_no_crc(&mut self) -> Result<u8> {
        let b = match self.rx.next() {
            Some(b) => b,
            None => bail!("End of data")
        };
        Ok(b)
    }

    fn read_4_bytes(&mut self) -> Result<[u8; 4]> {
        let buf = [
            self.read_byte_no_crc()?,
            self.read_byte_no_crc()?,
            self.read_byte_no_crc()?,
            self.read_byte_no_crc()?,
        ];
        Ok(buf)
    }

    fn initialized(&self) -> bool {
        self.init == 8
    }

    fn parse_init_seq(&mut self, b: u8) {
        if (b == 0x1b && self.init < 4) || (b == 0x01 && self.init >= 4) {
            self.init += 1;
        } else {
            self.num_discarded_bytes += 1 + self.init as usize;
            self.init = 0;
        }
    }

    fn push(&mut self, b: u8) -> Result<()> {
        if self.buf_len >= N {
            bail!("Buffer overflow")
        }
        self.buf[self.buf_len] = b;
        self.buf_len += 1;
        Ok(())
    }

    fn reset(&mut self) {
        self.buf_len = 0;
        self.init = 0;
        self.num_0x1b = 0;
        self.num_discarded_bytes = 0;
    }

    fn bail(&mut self, msg: &'static str) -> Result<()> {
        self.reset();
        bail!(msg)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn test_parse_input<const N: usize>(bytes: &[u8], exp: &[Result<ParseRes, ParseErr>]) {
        let mut sml_reader = SmlReader2::<N>::new();
        let mut exp_iter = exp.iter();
        
        for b in bytes {
            let res = sml_reader.push_byte(*b);
            match res {
                Ok(None) => {
                    // continue
                },
                Ok(Some(res)) => {
                    match exp_iter.next() {
                        Some(exp_res) => {
                            assert_eq!(Ok(res), *exp_res);
                        }
                        None => {
                            panic!("Additional ParseRes: {:?}", res);
                        }
                    }
                }
                Err(e) => {
                    match exp_iter.next() {
                        Some(exp_res) => {
                            assert_eq!(Err(e), *exp_res);
                        }
                        None => {
                            panic!("Additional Error: {:?}", e);
                        }
                    }
                }
            }
        }
        if let Some(final_res) = sml_reader.finalize() {
            assert_eq!(exp_iter.next(), Some(&Ok(final_res)));
        }
        assert_eq!(exp_iter.next(), None);
    }

    #[test]
    fn basic() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[
            Ok(ParseRes::Transmission(&hex!("12345678")))
        ];

        test_parse_input::<8>(&bytes, exp);
    }

    #[test]
    fn out_of_memory() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[
            Err(ParseErr::OutOfMemory)
        ];

        test_parse_input::<7>(&bytes, exp);
    }

    #[test]
    fn invalid_crc() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b8FF");
        let exp = &[
            Err(ParseErr::InvalidMessage {
                checksum_mismatch: (0xFFb8, 0x7bb8),
                end_esc_misaligned: false,
                num_padding_bytes: 0
            })
        ];

        test_parse_input::<8>(&bytes, exp);
    }

    #[test]
    fn msg_end_misaligned() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 FF 1b1b1b1b 1a0013b6");
        let exp = &[
            Err(ParseErr::InvalidMessage {
                checksum_mismatch: (0xb613, 0xb613),
                end_esc_misaligned: true,
                num_padding_bytes: 0,
            })
        ];

        test_parse_input::<16>(&bytes, exp);
    }

    #[test]
    fn padding_too_large() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 12345678 1b1b1b1b 1a04f950");
        let exp = &[
            Err(ParseErr::InvalidMessage {
                checksum_mismatch: (0x50f9, 0x50f9),
                end_esc_misaligned: false,
                num_padding_bytes: 4,
            })
        ];

        test_parse_input::<16>(&bytes, exp);
    }

    #[test]
    fn empty_msg_with_padding() {
        let bytes = hex!("1b1b1b1b 01010101 1b1b1b1b 1a014FF4");
        let exp = &[
            Err(ParseErr::InvalidMessage {
                checksum_mismatch: (0xf44f, 0xf44f),
                end_esc_misaligned: false,
                num_padding_bytes: 1,
            })
        ];

        test_parse_input::<16>(&bytes, exp);
    }

    #[test]
    fn additional_bytes() {
        let bytes = hex!("000102 1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b 1234");

        let exp = &[
            Ok(ParseRes::DiscardedBytes(3)),
            Ok(ParseRes::Transmission(&hex!("12345678"))),
            Ok(ParseRes::DiscardedBytes(2)),
        ];
        
        test_parse_input::<128>(&bytes, exp);
    }

    #[test]
    fn incomplete_message() {
        let bytes = hex!("1b1b1b1b 01010101 123456");

        let exp = &[
            Ok(ParseRes::DiscardedBytes(11)),
        ];
        
        test_parse_input::<128>(&bytes, exp);
    }

    #[test]
    fn padding() {
        let bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");

        let exp = &[
            Ok(ParseRes::Transmission(&hex!("123456"))),
        ];
        
        test_parse_input::<128>(&bytes, exp);
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");
        
        let exp = &[
            Ok(ParseRes::Transmission(&hex!("121b1b1b1b"))),
        ];
        
        test_parse_input::<128>(&bytes, exp);
    }

    #[test]
    fn real_data() {
        let bytes = hex!("1B1B1B1B010101017605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD55001B1B1B1B1A00D54B");
        
        let exp = &[
            Ok(ParseRes::Transmission(&hex!("7605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD5500"))),
        ];
        
        test_parse_input::<512>(&bytes, exp);
    }

    // TODO: test invalid esc sequences
    // TODO: test consecutive start messages


    // #[test]
    // fn easymeter() {
    //     let bytes = hex!("760B4553594199A502EA9AF2620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF6D7E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95B83570177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE590000000000012A160177070100240700FF0101621B52FE59000000000000BC0A0177070100380700FF0101621B52FE5900000000000016F401770701004C0700FF0101621B52FE5900000000000057180177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309150177070100340700FF0101622352FF6309060177070100480700FF0101622352FF6309150177078181C7F006FF010101010401071E0101016387C100760B4553594199A502EA9AF36200620072630201710163D3EC000000001B1B1B1B1A033A231B1B1B1B01010101760B4553594199A502EA9AF4620062007263010176010445535908455359DF6E9AF40B09014553591103B599A50101638CFF00760B4553594199A502EA9AF5620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF6E7E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95B8C640177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE5900000000000145BD0177070100240700FF0101621B52FE59000000000000C7580177070100380700FF0101621B52FE590000000000001D3E01770701004C0700FF0101621B52FE5900000000000061250177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309160177070100340700FF52FF6309070177070100480700FF0101622352FF6309160177078181C7F006FF010101010401071E010101638DA400760B4553594199A502EA9AF6620062007263020171016317E7000000001B1B1B1B1A037BD31B1B1B1B01010101760B4553594199A502EA9AF7620062007263010176010445535908455359DF6F9AF70B09014553591103B599A5010163620E00760B4553594199A502EA9AF8620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF6F7E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95B952E0177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE590000000000013C820177070100240700FF0101621B52FE59000000000000C55B0177070100380700FF0101621B52FE5900000000000018AF01770701004C0700FF0101621B52FE590000000000005E770177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309150177070100340700FF0101622352FF6309030177070100480700FF0101622352FF6309150177078181C7F006FF010101010401071E0101016362BC00760B4553594199A502EA9AF962006200726302017101635BFB000000001B1B1B1B1A03F8B11B1B1B1B01010101760B4553594199A502EA9AFA620062007263010176010445535908455359DF709AFA0B09014553591103B599A5010163216800760B4553594199A502EA9AFB620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF707E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95B9D760177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE590000000000012A140177070100240700FF0101621B52FE59000000000000BDEE0177070100380700FF0101621B52FE59000000000000142F01770701004C0700FF0101621B52FE5900000000000057F60177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309140177070100340700FF0101622352FF6309020177070100480700FF0101622352FF6309120177078181C7F006FF010101010401071E01010163250C00760B4553594199A502EA9AFC62006200726302017101639FF0000000001B1B1B1B1A03345C1B1B1B1B01010101760B4553594199A502EA9AFD620062007263010176010445535908455359DF719AFD0B09014553591103B599A501016368CB00760B4553594199A502EA9AFE620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF717E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95BA56C0177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE590000000000011E960177070100240700FF0101621B52FE59000000000000B9880177070100380700FF0101621B52FE59000000000000114501770701004C0700FF0101621B52FE5900000000000053C80177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309140177070100340700FF0101622352FF6309050177070100480700FF0101622352FF6309130177078181C7F006FF010101010401071E01010163DC3200760B4553594199A502EA9AFF62006200726302017101632C0E000000001B1B1B1B1A0302F51B1B1B1B01010101760B4553594199A502EA9B00620062007263010176010445535908455359A5010163884700760B4553594199A502EA9B01620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF727E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95BAD320177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE5900000000000117DF0177070100240700FF0101621B52FE59000000000000B7960177070100380700FF0101621B52FE590000000000000EF901770701004C0700FF0101621B52FE5900000000000051500177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309130177070100340700FF0101622352FF6309050177070100480700FF0101622352FF6309150177078181C7F006FF010101010401071E01010163060A00760B4553594199A502EA9B026200620072630201710163C6F0000000001B1B1B1B1A03F9081B1B1B1B01010101760B4553594199A502EA9B03620062007263010176010445535908455359DF739B030B09014553591103B599A501016366B600760B4553594199A502EA9B04620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF737E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95BB4D30177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE5900000000000112A40177070100240700FF0101621B52FE59000000000000B5960177070100380700FF0101621B52FE590000000000000D9501770701004C0700FF0101621B52FE590000000000004F780177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309120177070100340700FF0101622352FF6309020177070100480700FF0101622352FF6309130177078181C7F006FF010101010401071E01010163B5F600760B4553594199A502EA9B0562006200726302017101632050000000001B1B1B1B1A032D911B1B1B1B01010101760B4553594199A502EA9B06620062007263010176010445535908455359DF749B060B09014553591103B599A5010163278400760B4553594199A502EA9B07620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF747E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95BBC4A0177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE590000000000010CB20177070100240700FF0101621B52FE59000000000000B3050177070100380700FF0101621B52FE590000000000000CA801770701004C0700FF0101621B52FE590000000000004D040177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309120177070100340700FF0101622352FF6309010177070100480700FF0101622352FF6309100177078181C7F006FF010101010401071E01010163160D00760B4553594199A502EA9B0862006200726302017101634EE7000000001B1B1B1B1A03A2221B1B1B1B01010101760B4553594199A502EA9B09620062007263010176010445535908455359DF759B090B09014553591103B599A5010163208200760B4553594199A502EA9B0A620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF757E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B5");
    //     //let bytes = hex!("1B1B1B1B01010101760B4553594199A502EA9AF4620062007263010176010445535908455359DF6E9AF40B09014553591103B599A50101638CFF00760B4553594199A502EA9AF5620062007263070177010B09014553591103B599A5070100620AFFFF7262016500F8DF6E7E77078181C78203FF01010101044553590177070100000009FF010101010B09014553591103B599A50177070100010800FF6400008001621E52FC5900000006D95B8C640177070100020800FF6400008001621E52FC5900000000419BD4D30177070100100700FF0101621B52FE5900000000000145BD0177070100240700FF0101621B52FE59000000000000C7580177070100380700FF0101621B52FE590000000000001D3E01770701004C0700FF0101621B52FE5900000000000061250177078181C78205FF0101010183021A24687A277E98565E1093055BEE0F704E58FDAA3DD19D4FAF3EE067C164C30494DAE9EA1566ED727D236AAF5AB09A5B0177070100000000FF010101010F31455359313136323233323939370177070100200700FF0101622352FF6309160177070100340700FF52FF6309070177070100480700FF0101622352FF6309160177078181C7F006FF010101010401071E010101638DA400760B4553594199A502EA9AF6620062007263020171016317E7000000001B1B1B1B1A037BD3");
    //     // for i in 0..10 {
    //     //     let mut crc = crc16::Digest::new(crc16::X25);
    //     //     crc.write(&bytes[..bytes.len()-i]);
    //     //     println!("{}: {}", i, crc.sum16());
    //     // }
    //     let mut sml_reader = SmlReader::<_, 2024>::new(bytes.iter().cloned());
    //     for i in 0..10 {
    //         let res = sml_reader.read_transmission_into_slice();
    //         match res {
    //             Ok(b) => {
    //                 let f = crate::FileIter::new(b);
    //                 println!("{}: OK. Messages:", i);
    //                 for msg in f {
    //                     println!("  {}", if msg.is_ok() { "OK"} else { "error" });
    //                     if let Ok(msg) = msg {
    //                         let body = msg.message_body;
    //                         if let MessageBody::GetListResponse(glr) = body {
    //                             for val in glr.val_list {
    //                                 println!("    {:?}", val);
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //             Err(e) => {
    //                 println!("{}: Err: {}", i, e);
    //             }
    //         }
            
    //         // if let Ok(bytes) = res {
    //         //     let f = crate::FileIter::new(bytes);
    //         //     for msg in f {
    //         //         println!("{:?}", msg);
    //         //     }
    //         // }
    //     }
    //     assert!(false);
    // }
}