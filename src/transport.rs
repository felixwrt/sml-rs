use crate::CRC_X25;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EncoderState {
    Init0,
    Init1,
    Init2,
    Init3,
    Init4,
    Init5,
    Init6,
    Init7,
    Read0,
    Read1,
    Read2,
    Read3,
    Esc0,
    Esc1,
    Esc2,
    Esc3,
    Pad3,
    Pad2,
    Pad1,
    End0,
    End1,
    End2,
    End3,
    End4,
    NPad,
    Crc1,
    Crc2,
    Done,
}

impl EncoderState {
    fn from_padding(pad: u8) -> Self {
        use EncoderState::*;
        match pad {
            3 => Pad3,
            2 => Pad2,
            1 => Pad1,
            _ => End0
        }
    }
}

pub struct Encoder<I> 
where 
    I: Iterator<Item = u8>
{
    state: EncoderState,
    crc: crc::Digest<'static, u16>,
    padding: Padding,
    iter: I
}

impl<I> Encoder<I>
where
    I: Iterator<Item = u8>
{
    pub fn new(iter: I) -> Self {
        let mut crc = CRC_X25.digest();
        crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01]);
        Encoder { 
            state: EncoderState::Init0,
            crc,
            padding: Padding::new(),
            iter 
        }
    }

    fn read_from_iter(&mut self) -> Option<u8> {
        let ret = self.iter.next();
        if ret.is_some() {
            self.padding.bump();
        }
        ret
    }

    #[inline(always)]
    pub fn next_state(&self) -> EncoderState {
        use EncoderState::*;
        if self.state == Done {
            return Done;
        }
        if self.state == Esc3 {
            return Read0;
        }
        unsafe { std::mem::transmute(self.state as u8 + 1) }
        // match self.state {
        //     Init0 => Init1,
        //     Init1 => Init2,
        //     Init2 => Init3,
        //     Init3 => Init4,
        //     Init4 => Init5,
        //     Init5 => Init6,
        //     Init6 => Init7,
        //     Init7 => Read0,
        //     Read0 => Read1,
        //     Read1 => Read2,
        //     Read2 => Read3,
        //     Read3 => Esc0,
        //     Esc0 => Esc1,
        //     Esc1 => Esc2,
        //     Esc2 => Esc3,
        //     Esc3 => Read0,
        //     Pad3 => Pad2,
        //     Pad2 => Pad1,
        //     Pad1 => End0,
        //     End0 => End1,
        //     End1 => End2,
        //     End2 => End3,
        //     End3 => End4,
        //     End4 => NPad,
        //     NPad => Crc1,
        //     Crc1 => Crc2,
        //     Crc2 => Done,
        //     Done => Done,
        // }
    }
}

impl<I> Iterator for Encoder<I>
where
    I: Iterator<Item = u8>
{
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        use EncoderState::*;
        let out = match self.state {
            Init0 | Init1 | Init2 | Init3 => {
                0x1b
            }
            Init4 | Init5 | Init6 | Init7 => {
                0x01
            }
            Read0 | Read1 | Read2 | Read3 => {
                match self.read_from_iter() {
                    Some(b) => {
                        self.crc.update(&[b]);
                        if b != 0x1b {
                            self.state = Read0;
                            return Some(b);
                        }
                        b
                    }
                    None => {
                        let padding = self.padding.get();
                        // finalize crc
                        for _ in 0..padding {
                            self.crc.update(&[0x00]);
                        }
                        self.crc.update(&[0x1b, 0x1b, 0x1b, 0x1b, 0x1a, padding]);
                        
                        self.state = EncoderState::from_padding(padding);
                        return self.next();
                    }
                }
            }
            Esc0 | Esc1 | Esc2 | Esc3 => {
                self.crc.update(&[0x1b]);
                0x1b
            }
            Pad3 | Pad2 | Pad1 => {
                0x00
            }
            End0 | End1 | End2 | End3 => {
                0x1b
            }
            End4 => {
                0x1a
            }
            NPad => {
                self.padding.get()
            }
            s @ (Crc1 | Crc2) => {
                let crc_bytes: [u8; 2] = self.crc.clone().finalize().to_le_bytes();
                let idx = (s == Crc2) as usize;
                crc_bytes[idx]
            }
            Done => {
                return None;
            }
        };
        self.state = self.next_state();
        Some(out)
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

pub fn foo() {
    let v = [1,2,3,4,5,6];
    let out: Vec<_> = Encoder::new(v.into_iter()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn test_encoding(bytes: &[u8], exp_encoded_bytes: &[u8]) {
        compare_encoded_bytes(
            exp_encoded_bytes,
            &encode_v1(&bytes)
        );
        compare_encoded_bytes(
            exp_encoded_bytes, 
            &Encoder::new(bytes.into_iter().cloned()).collect::<Vec<_>>(),
        );
    }

    fn compare_encoded_bytes(expected: &[u8], actual: &[u8]) {
        if expected != actual {
            // use strings here such that the output uses hex formatting
            assert_eq!(
                format!("{:02x?}", expected),
                format!("{:02x?}", actual),
            );
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
