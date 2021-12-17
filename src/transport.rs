use anyhow::Result;

extern crate alloc;
use core::{convert::TryInto, ops::Deref};

static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

pub type ArrayBuf<const N: usize> = heapless::Vec<u8, N>;
pub type VecBuf = alloc::vec::Vec<u8>;

pub trait Buffer: Default + Deref<Target=[u8]> {
    fn push(&mut self, b: u8) -> Result<(), u8>;

    fn truncate(&mut self, len: usize);
    
    fn clear(&mut self);
}

impl<const N: usize> Buffer for ArrayBuf<N> {
    fn push(&mut self, b: u8) -> Result<(), u8> {
        ArrayBuf::push(self, b)
    }

    fn truncate(&mut self, len: usize) {
        ArrayBuf::truncate(self, len)
    }

    fn clear(&mut self) {
        ArrayBuf::clear(self)
    }
}

impl Buffer for VecBuf {
    fn push(&mut self, b: u8) -> Result<(), u8> {
        match self.try_reserve(1) {
            Ok(()) => {
                VecBuf::push(self, b);
                Ok(())
            }
            Err(_) => Err(b)
        }
        
    }

    fn truncate(&mut self, len: usize) {
        VecBuf::truncate(self, len)
    }

    fn clear(&mut self) {
        VecBuf::clear(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseRes<T> {
    DiscardedBytes(usize),  // just found the start of a transmission, but some previous bytes could not be parsed
    Transmission(T),  // a full & valid transmission has been read. These are the bytes that make the message
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseErr {
    InvalidEsc([u8; 4]),  // an invalid escape sequence has been read
    OutOfMemory,  // the buffer used internally is full. When using vec, allocation has failed
    InvalidMessage {
        checksum_mismatch: (u16, u16),  // (expected, found)
        end_esc_misaligned: bool,
        num_padding_bytes: u8,
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

pub struct SmlReader<B: Buffer> {
    buf: B,
    raw_msg_len: usize,
    crc: crc::Digest<'static, u16>,
    crc_idx: usize,
    state: ParseState
}

impl<B: Buffer> SmlReader<B> {
    pub fn new() -> Self {
        Self::from_buf(Default::default())
    }

    pub fn from_buf(buf: B) -> Self {
        SmlReader {
            buf: buf,
            raw_msg_len: 0,
            crc: CRC_X25.digest(),
            crc_idx: 0,
            state: ParseState::LookingForMessageStart {
                num_discarded_bytes: 0,
                num_init_seq_bytes: 0,
            }
        }
    }

    pub fn push_byte(&mut self, b: u8) -> Result<Option<ParseRes<&[u8]>>, ParseErr> {
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
                    self.raw_msg_len = 8;
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
                    } else if payload == &[0x01, 0x01, 0x01, 0x01] {
                        // another transmission start

                        // ignore everything that has previously been read and start reading a new transmission
                        let ignored_bytes = self.raw_msg_len - 8;
                        self.raw_msg_len = 8;
                        self.buf.clear();
                        self.crc = CRC_X25.digest();
                        self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
                        self.crc_idx = 0;
                        self.state = ParseState::ParsingNormal;
                        return Ok(Some(ParseRes::DiscardedBytes(ignored_bytes)))
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

    pub fn finalize(self) -> Option<ParseRes<&'static [u8]>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn test_parse_input<B: Buffer>(bytes: &[u8], exp: &[Result<ParseRes<&[u8]>, ParseErr>]) {
        let mut sml_reader = SmlReader::<B>::new();
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
            Ok(ParseRes::Transmission(hex!("12345678").as_slice()))
        ];

        test_parse_input::<ArrayBuf<8>>(&bytes, exp);
    }

    #[test]
    fn out_of_memory() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[
            Err(ParseErr::OutOfMemory)
        ];

        test_parse_input::<ArrayBuf<7>>(&bytes, exp);
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

        test_parse_input::<ArrayBuf<8>>(&bytes, exp);
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

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
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

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
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

        test_parse_input::<ArrayBuf<16>>(&bytes, exp);
    }

    #[test]
    fn additional_bytes() {
        let bytes = hex!("000102 1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b 1234");

        let exp = &[
            Ok(ParseRes::DiscardedBytes(3)),
            Ok(ParseRes::Transmission(hex!("12345678").as_slice())),
            Ok(ParseRes::DiscardedBytes(2)),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn incomplete_message() {
        let bytes = hex!("1b1b1b1b 01010101 123456");

        let exp = &[
            Ok(ParseRes::DiscardedBytes(11)),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn invalid_esc_sequence() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1c000000 12345678 1b1b1b1b 1a03be25");
        
        let exp = &[
            Err(ParseErr::InvalidEsc([0x1c, 0x0, 0x0, 0x0])),
            Ok(ParseRes::DiscardedBytes(12)),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn double_msg_start() {
        let bytes = hex!("1b1b1b1b 01010101 09 87654321 1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        
        let exp = &[
            Ok(ParseRes::DiscardedBytes(13)),
            Ok(ParseRes::Transmission(hex!("12345678").as_slice())),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn padding() {
        let bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");

        let exp = &[
            Ok(ParseRes::Transmission(hex!("123456").as_slice())),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");
        
        let exp = &[
            Ok(ParseRes::Transmission(hex!("121b1b1b1b").as_slice())),
        ];
        
        test_parse_input::<ArrayBuf<128>>(&bytes, exp);
    }

    #[test]
    fn real_data() {
        let bytes = hex!("1B1B1B1B010101017605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD55001B1B1B1B1A00D54B");
        
        let exp = &[
            Ok(ParseRes::Transmission(hex!("7605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD5500").as_slice())),
        ];
        
        test_parse_input::<ArrayBuf<512>>(&bytes, exp);
    }

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

    #[test]
    fn alloc_basic() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let exp = &[
            Ok(ParseRes::Transmission(hex!("12345678").as_slice()))
        ];

        test_parse_input::<VecBuf>(&bytes, exp);
    }
}
