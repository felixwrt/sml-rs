#![allow(unused)]
#![allow(missing_docs)]
use crate::{transport::{DecodeErr, DecoderInner, ReadDecodedError}, util::{Buffer, SliceBuf}};

pub struct IoReader<R: std::io::Read> {
    reader: R,
    decoder: DecoderInner
}

impl<R: std::io::Read> IoReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            decoder: Default::default(),
        }
    }

    pub fn read_message_into_slice(&mut self, buf: &mut [u8]) -> Result<usize, ReadDecodedError<std::io::Error>> {
        let mut buf = SliceBuf::new(buf);
        let buf = &mut buf;
        loop {
            let mut b = 0u8;
            match self.reader.read(std::slice::from_mut(&mut b)) {
                Ok(1) => (),
                Ok(_) => unreachable!(),
                Err(e) => {
                    let num_discarded_bytes = match self.decoder.finalize(buf) {
                        Some(DecodeErr::DiscardedBytes(n)) => n,
                        Some(_) => unreachable!(),
                        None => 0,
                    };
                    return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
                },
            }
            match self.decoder.push_byte(buf, b) {
                Ok(false) => continue,
                Ok(true) => return Ok(buf.len()),
                Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
            }
        }
    }

    pub fn read_message_into_vec(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadDecodedError<std::io::Error>> {
        buf.clear();
        loop {
            let mut b = 0u8;
            match self.reader.read(std::slice::from_mut(&mut b)) {
                Ok(1) => (),
                Ok(_) => unreachable!(),
                Err(e) => {
                    let num_discarded_bytes = match self.decoder.finalize(buf) {
                        Some(DecodeErr::DiscardedBytes(n)) => n,
                        Some(_) => unreachable!(),
                        None => 0,
                    };
                    return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
                },
            }
            match self.decoder.push_byte(buf, b) {
                Ok(false) => continue,
                Ok(true) => return Ok(()),
                Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
            }
        }
    }
}

#[test]
fn test_passing_non_empty_vec() {
    use hex_literal::hex;
    let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
    let exp = hex!("12345678");
    let mut reader = std::io::Cursor::new(bytes);
    let mut reader = IoReader::new(reader);
    let mut v = vec!(1, 2, 3);
    let ret = reader.read_message_into_vec(&mut v);
    assert!(ret.is_ok());
    assert_eq!(v, exp);
}