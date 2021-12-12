use crc::{crc16, Hasher16};
use anyhow::{Result, bail};

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
                                    self.is_active = false;
                                    let msg = self.buf.clone();
                                    self.state = ReaderState::LookForEsc(0);
                                    return Ok((msg, self.buf_len));
                                } else if esc_bytes == &[0x01, 0x01, 0x01, 0x01] {
                                    // start sequence
                                    self.is_active = true;
                                } else if !self.is_active {
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

pub struct SmlReader<Rx, const N: usize> 
where
    Rx: Iterator<Item=u8>
{
    rx: Rx,
    init: u8,
    num_0x1b: u8,
    buf: [u8; N],
    buf_len: usize,
    crc: crc16::Digest
}

impl<Rx, const N: usize> SmlReader<Rx, N>
where
    Rx: Iterator<Item=u8>
{
    pub fn new(rx: Rx) -> Self {
        SmlReader {
            rx: rx,
            init: 0,
            num_0x1b: 0,
            buf: [0; N],
            buf_len: 0,
            crc: crc16::Digest::new(crc16::X25),
        }
    }

    pub fn read_transmission(mut self) -> Result<([u8;N], usize)> {
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
                        for b in bytes {
                            self.push(b)?;
                        }
                        self.num_0x1b = 0;
                        self.crc.write(&bytes);
                    } else if bytes[0] == 0x1a {
                        // end of transmission
                        let num_padding_bytes = bytes[1];
                        if num_padding_bytes > 3 {
                            bail!("Invalid number of padding bytes")
                        }
                        self.crc.write(&bytes[..2]);
                        let checksum = u16::from_le_bytes([bytes[2], bytes[3]]);
                        if self.crc.sum16() != checksum {
                            bail!("Checksum doesn't match")
                        }
                        
                        self.buf_len -= num_padding_bytes as usize;
                        return Ok((self.buf, self.buf_len))
                    } else {
                        bail!("Invalid start sequence read")
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
        self.crc.write(&[b]);
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
            if self.init > 0 {
                self.crc = crc16::Digest::new(crc16::X25);
            }
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
}


#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn basic() {
        let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let sml_reader = SmlReader::<_, 128>::new(bytes.iter().cloned());

        let (buf, len) = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&buf[..len], &hex!("12345678"));
    }

    #[test]
    fn padding() {
        let bytes = hex!("1b1b1b1b 01010101 12345600 1b1b1b1b 1a0191a5");
        let sml_reader = SmlReader::<_, 128>::new(bytes.iter().cloned());

        let (buf, len) = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&buf[..len], &hex!("123456"));
    }

    #[test]
    fn escape_in_user_data() {
        let bytes = hex!("1b1b1b1b 01010101 12 1b1b1b1b 1b1b1b1b 000000 1b1b1b1b 1a03be25");
        let sml_reader = SmlReader::<_, 128>::new(bytes.iter().cloned());

        let (buf, len) = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&buf[..len], &hex!("121b1b1b1b"));
    }

    #[test]
    fn real_data() {
        let bytes = hex!("1B1B1B1B010101017605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD55001B1B1B1B1A00D54B");
        let sml_reader = SmlReader::<_, 256>::new(bytes.iter().cloned());

        let (buf, len) = sml_reader.read_transmission().expect("Parsing failed");

        assert_eq!(&buf[..len], &hex!("7605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD5500"));
    }
}