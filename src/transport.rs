use crate::CRC_X25;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

struct Padding(u8);

impl Padding {
    fn new() -> Self {
        Padding(0)
    }

    fn bump(&mut self) {
        self.0 = self.0.wrapping_sub(1);
    }

    fn get(&self) -> u8 {
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
                        (Some(b), LookingForEscape((n + 1) * (b == 0x1b) as u8))
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

#[cfg(feature = "alloc")]
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

    #[cfg(not(feature = "alloc"))]
    fn test_encoding(bytes: &[u8], exp_encoded_bytes: &[u8]) {
        let mut encoder = Encoder::new(bytes.into_iter().cloned());
        for exp_byte in exp_encoded_bytes {
            assert_eq!(Some(*exp_byte), encoder.next());
        }
        assert_eq!(None, encoder.next());
    }

    #[cfg(feature = "alloc")]
    fn test_encoding(bytes: &[u8], exp_encoded_bytes: &[u8]) {
        compare_encoded_bytes(exp_encoded_bytes, &encode_v1(&bytes));
        compare_encoded_bytes(
            exp_encoded_bytes,
            &Encoder::new(bytes.into_iter().cloned()).collect::<Vec<_>>(),
        );
    }

    #[cfg(feature = "alloc")]
    fn compare_encoded_bytes(expected: &[u8], actual: &[u8]) {
        if expected != actual {
            // use strings here such that the output uses hex formatting
            assert_eq!(format!("{:02x?}", expected), format!("{:02x?}", actual),);
        }
    }

    #[test]
    fn basic() {
        test_encoding(
            &hex!("12345678"),
            &hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b"),
        );
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
