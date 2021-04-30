use std::io::Read;
use crc::{crc16, Hasher16};


pub struct SmlReader<R: Read> {
    inner: R,
    init: u8,
    num_0x1b: u8,
    buf: Vec<u8>,
    crc: crc16::Digest
}

impl<R: Read> SmlReader<R> {
    pub fn new(inner: R) -> SmlReader<R> {
        SmlReader {
            inner: inner,
            init: 0,
            num_0x1b: 0,
            buf: Vec::new(),
            crc: crc16::Digest::new(crc16::X25),
        }
    }

    pub fn read_transmission(mut self) -> Result<Vec<u8>, std::io::Error> {
        while !self.initialized() {
            let b = self.read_byte()?;
            self.parse_init_seq(b);
        }

        loop {
            let b = self.read_byte()?;
            if b == 0x1b {
                self.num_0x1b += 1;
                if self.num_0x1b == 4 {
                    // escape sequence found
                    let bytes = self.read_4_bytes()?;
                    
                    if bytes == [0x1b, 0x1b, 0x1b, 0x1b] {
                        // escape sequence in user data
                        self.buf.extend_from_slice(&bytes);
                        self.num_0x1b = 0;
                        self.crc.write(&bytes);
                    } else if bytes[0] == 0x1a {
                        // end of transmission
                        let num_padding_bytes = bytes[1];
                        if num_padding_bytes > 3 {
                            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid number of padding bytes"));
                        }
                        self.crc.write(&bytes[..2]);
                        let checksum = u16::from_le_bytes([bytes[2], bytes[3]]);
                        println!("calc: {:x}, exp: {:x}", self.crc.sum16(), checksum);
                        if self.crc.sum16() != checksum {
                            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Checksum doesn't match"));
                        }
                        
                        self.buf.truncate(self.buf.len() - num_padding_bytes as usize);
                        return Ok(self.buf)
                    } else {
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid start sequence read"));
                    }
                }
            } else {
                // no escape sequence, so add delayed values
                for _ in 0..self.num_0x1b {
                    self.buf.push(0x1b);
                }
                self.num_0x1b = 0;
                self.buf.push(b);
            }
        }
    }

    fn read_byte(&mut self) -> Result<u8, std::io::Error> {
        let mut buf = [0; 1];
        self.inner.read_exact(&mut buf)?;
        self.crc.write(&buf);
        Ok(buf[0])
    }

    fn read_4_bytes(&mut self) -> Result<[u8; 4], std::io::Error> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }


    fn initialized(&self) -> bool {
        self.init == 8
    }

    fn parse_init_seq(&mut self, b: u8) {
        if (b == 0x1b && self.init < 4) || (b == 0x01 && self.init >= 4) {
            self.init += 1;
        } else {
            if self.init > 0 {
                self.crc = crc16::Digest::new(crc16::X25);
            }
            self.init = 0;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn basic() {
        
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let cursor = std::io::Cursor::new(bytes);
        let sml_reader = SmlReader::new(cursor);

        let transmission = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&transmission, &hex!("12345678"));
    }

    #[test]
    fn padding() {
        let bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");
        let cursor = std::io::Cursor::new(bytes);
        let sml_reader = SmlReader::new(cursor);

        let transmission = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&transmission, &hex!("123456"));
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");
        let cursor = std::io::Cursor::new(bytes);
        let sml_reader = SmlReader::new(cursor);

        let transmission = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&transmission, &hex!("121b1b1b1b"));
    }

    #[test]
    fn real_data() {
        let bytes = hex!("1B1B1B1B010101017605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD55001B1B1B1B1A00D54B");

        let cursor = std::io::Cursor::new(bytes);
        let sml_reader = SmlReader::new(cursor);

        let transmission = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&transmission, &hex!("7605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD5500"));
    }
}