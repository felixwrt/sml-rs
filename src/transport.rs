use crate::CRC_X25;

enum EncoderState {
    Init(u8),
    LookingForEscape(u8),
    HandlingEscape(u8),
    Padding(u8, u8),
    EndEscape(u8, u8),
    Crc(u8),
    Done,
}

pub struct Encoder<I> 
where 
    I: Iterator<Item = u8>
{
    state: EncoderState,
    crc: crc::Digest<'static, u16>,
    padding: u8,
    iter: I
}

impl<I> Encoder<I>
where
    I: Iterator<Item = u8>
{
    pub fn new(iter: I) -> Self {
        Encoder { 
            state: EncoderState::Init(0), 
            crc: CRC_X25.digest(), 
            padding: 0,
            iter 
        }
    }

    fn next_(&mut self) -> Option<u8> {
        match self.state {
            EncoderState::Init(ref mut n) if *n < 4 => {
                *n += 1;
                Some(0x1b)
            }
            EncoderState::Init(ref mut n) if *n < 8 => {
                *n += 1;
                Some(0x01)
            }
            EncoderState::Init(n) => {
                assert_eq!(n, 8);
                self.state = EncoderState::LookingForEscape(0);
                self.next_()
            }
            EncoderState::LookingForEscape(ref mut n) if *n < 4 => {
                match self.iter.next() {
                    Some(0x1b) => {
                        *n += 1;
                        Some(0x1b)
                    }
                    Some(b) => {
                        *n = 0;
                        Some(b)
                    }
                    None => {
                        self.state = EncoderState::Padding(self.padding % 4, self.padding % 4);
                        self.next_()
                    }
                }
            }
            EncoderState::LookingForEscape(n) => {
                assert_eq!(n, 4);
                self.state = EncoderState::HandlingEscape(0);
                self.next_()
            }
            EncoderState::HandlingEscape(ref mut n) if *n < 4 => {
                *n += 1;
                Some(0x1b)
            }
            EncoderState::HandlingEscape(n) => {
                assert_eq!(n, 4);
                self.state = EncoderState::LookingForEscape(0);
                self.next_()
            }
            EncoderState::Padding(ref mut n, _pad) if *n > 0 => {
                *n -= 1;
                Some(0x00)
            }
            EncoderState::Padding(n, pad) => {
                assert_eq!(n, 0);
                self.state = EncoderState::EndEscape(0, pad);
                self.next_()
            }
            EncoderState::EndEscape(ref mut n, _pad) if *n < 4 => {
                *n += 1;
                Some(0x1b)
            }
            EncoderState::EndEscape(ref mut n, _pad) if *n == 4 => {
                *n += 1;
                Some(0x1a)
            }
            EncoderState::EndEscape(ref mut n, pad) if *n == 5 => {
                *n += 1;
                Some(pad)
            }
            EncoderState::EndEscape(n, _pad) => {
                assert_eq!(n, 6);
                let crc_bytes = self.crc.clone().finalize().to_le_bytes();
                self.state = EncoderState::Crc(crc_bytes[1]);
                Some(crc_bytes[0])
            }
            EncoderState::Crc(b) => {
                self.state = EncoderState::Done;
                Some(b)
            }
            EncoderState::Done => {
                None
            }
        }
    }
}

impl<I> Iterator for Encoder<I>
where
    I: Iterator<Item = u8>
{
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let out = self.next_();
        if let Some(b) = out {
            self.crc.update(&[b]);
            self.padding = self.padding.wrapping_sub(1);
        }
        out
    }
}

pub fn encode_v1(bytes: &[u8]) -> Vec<u8> {
    // start escape sequence
    let mut res = vec![0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01];

    // encode data
    let mut num_1b = 0;
    for b in bytes {
        if *b == 0x1b {
            num_1b += 1;
        } else {
            num_1b = 0;
        }

        res.push(*b);

        if num_1b == 4 {
            res.extend([0x1b; 4]);
            num_1b = 0;
        }
    }

    // padding bytes
    let num_padding_bytes = (4 - (res.len() % 4)) % 4;
    res.resize(res.len() + num_padding_bytes, 0x0);

    res.extend([0x1b, 0x1b, 0x1b, 0x1b, 0x1a, num_padding_bytes as u8]);
    let crc = CRC_X25.checksum(res.as_slice());

    res.extend(crc.to_le_bytes());

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn basic() {
        let bytes = hex!("12345678");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
        assert_eq!(Encoder::new(bytes.into_iter()).collect::<Vec<_>>(), encoded_bytes);
    }

    #[test]
    fn padding() {
        let bytes = hex!("123456");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
        assert_eq!(Encoder::new(bytes.into_iter()).collect::<Vec<_>>(), encoded_bytes);
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("121b1b1b1b");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
        assert_eq!(Encoder::new(bytes.into_iter()).collect::<Vec<_>>(), encoded_bytes);
    }

    #[test]
    fn almost_escape_in_user_data() {
        let bytes = hex!("121b1b1b1a");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1a 000000 1b1b1b1b 1a03a71a");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
        assert_eq!(Encoder::new(bytes.into_iter()).collect::<Vec<_>>(), encoded_bytes);
    }
}
