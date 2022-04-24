use crate::CRC_X25;

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
    }

    #[test]
    fn padding() {
        let bytes = hex!("123456");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("121b1b1b1b");
        let encoded_bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");

        assert_eq!(encode_v1(&bytes), encoded_bytes);
    }
}
