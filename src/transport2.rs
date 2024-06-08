#![allow(unused)]
#![allow(missing_docs)]
use crate::{transport::{DecodeErr, DecoderInner, ReadDecodedError}, util::{Buffer, SliceBuf}};

pub struct IoReader<R: std::io::Read> {
    reader: R,
    decoder: DecoderInner
}

impl<R: std::io::Read> IoReader<R> {
    
    fn read_message_into_slice(&mut self, buf: &mut [u8]) -> Result<usize, ReadDecodedError<std::io::Error>> {
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
                }, // TODO: real number of discarded bytes?
            }
            match self.decoder.push_byte(buf, b) {
                Ok(false) => continue,
                Ok(true) => return Ok(buf.len()),
                Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
            }
        }
    }

    fn read_message_into_vec(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadDecodedError<std::io::Error>> {
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
                }, // TODO: real number of discarded bytes?
            }
            match self.decoder.push_byte(buf, b) {
                Ok(false) => continue,
                Ok(true) => return Ok(()),
                Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
            }
        }
    }
}
