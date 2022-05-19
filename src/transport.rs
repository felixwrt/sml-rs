use crate::CRC_X25;

pub struct Encoder<I> 
where 
    I: Iterator<Item = u8>
{
    state: u8,
    crc: crc::Digest<'static, u16>,
    padding: u8,
    end_sequence: [u8; 11],
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
            state: 8, 
            crc, 
            padding: 0,
            end_sequence: [0x00, 0x00, 0x00, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0xFF, 0xFF, 0xFF],
            iter 
        }
    }
}

impl<I> Iterator for Encoder<I>
where
    I: Iterator<Item = u8>
{
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.state < 4 {
            match self.iter.next() {
                Some(b) => {
                    self.padding = self.padding.wrapping_sub(1);
                    self.state = (self.state + 1) * (b==0x1b) as u8;
                    self.crc.update(&[b]);
                    return Some(b);
                }
                None => {
                    // end of the input
                    let padding = self.padding & 0x03;
                    self.end_sequence[8] = padding;
                    let offset = 3 - padding;
                    self.crc.update(&self.end_sequence[offset as usize..(self.end_sequence.len()-2)]);
                    let crc = self.crc.clone().finalize().to_le_bytes();
                    self.end_sequence[9] = crc[0];
                    self.end_sequence[10] = crc[1];

                    self.state = 16 + offset;
                    return self.next();
                }
            }
        } else if self.state < 8 {
            // handling escape
            self.crc.update(&[0x1b]);
            self.state = (self.state + 1) * (self.state != 7) as u8;
            return Some(0x1b);
        } else if self.state < 16 {
            // init sequence
            let r = 0x01 + 0x1a * (self.state < 12) as u8;
            self.state = (self.state + 1) * (self.state != 15) as u8;
            return Some(r);
        } else if self.state < (16 + 11) {
            // end sequence
            let r = self.end_sequence[self.state as usize - 16];
            self.state += 1;
            return Some(r);
        } else {
            return None;
        }
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
