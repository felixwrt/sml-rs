#![allow(unused)]
#![allow(missing_docs)]
use core::borrow::Borrow;

use crate::{transport::{DecodeErr, DecoderInner, ReadDecodedError}, util::{Buffer, Eof, SliceBuf}};


// GOAL:
// support the following reader apis:
// - std::io::Read:
//   - fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>;
//   - NOTE: usually, this will be blocking. Even though non-blocking is possible, I'm going to ignore that case for now
// - embedded_io::Read:
//   - type Error;
//   - fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
//   - fn read_ready(&mut self) -> Result<bool, Self::Error>;
//   - NOTE: can be blocking or non-blocking and there should be APIs for both use-cases.
// - embedded_io_async::Read:
//   - async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
//   - NOTE: async fn!
// - Iterator<Item=u8>:
//   - fn next(&mut self) -> Option<u8>;

#[cfg(feature = "std")]
pub struct IoReader<R: std::io::Read> {
    reader: R,
    decoder: DecoderInner,
}

#[cfg(feature = "std")]
impl<R: std::io::Read> IoReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            decoder: DecoderInner::new(),
        }
    }

    pub fn read_message(&mut self, buf: &mut impl Buffer) -> Result<usize, ReadDecodedError<std::io::Error>> {
        todo!()
    }

    #[cfg(feature = "nb")]
    fn read_message_nb(&mut self, buf: &mut impl Buffer) -> nb::Result<(), std::io::Error> {
        todo!()
    }    

    // pub fn read_message_into_slice(&mut self, buf: &mut [u8]) -> Result<usize, ReadDecodedError<std::io::Error>> {
    //     let mut buf = SliceBuf::new(buf);
    //     let buf = &mut buf;
    //     loop {
    //         let mut b = 0u8;
    //         match self.reader.read(std::slice::from_mut(&mut b)) {
    //             Ok(1) => (),
    //             Ok(_) => unreachable!(),
    //             Err(e) => {
    //                 let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                     Some(DecodeErr::DiscardedBytes(n)) => n,
    //                     Some(_) => unreachable!(),
    //                     None => 0,
    //                 };
    //                 return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
    //             },
    //         }
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(buf.len()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }

    // pub fn read_message_into_vec(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadDecodedError<std::io::Error>> {
    //     buf.clear();
    //     loop {
    //         let mut b = 0u8;
    //         match self.reader.read(std::slice::from_mut(&mut b)) {
    //             Ok(1) => (),
    //             Ok(_) => unreachable!(),
    //             Err(e) => {
    //                 let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                     Some(DecodeErr::DiscardedBytes(n)) => n,
    //                     Some(_) => unreachable!(),
    //                     None => 0,
    //                 };
    //                 return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
    //             },
    //         }
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }
}

#[cfg(feature = "embedded-io")]
pub struct EIoReader<R: embedded_io::Read> {
    reader: R,
    decoder: DecoderInner,
}

#[cfg(feature = "embedded-io")]
impl<R: embedded_io::Read> EIoReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            decoder: Default::default(),
        }
    }

    pub fn read_message(&mut self, buf: &mut impl Buffer) -> Result<usize, ReadDecodedError<R::Error>> {
        todo!()
    }

    #[cfg(feature = "nb")]
    fn read_message_nb(&mut self, buf: &mut impl Buffer) -> nb::Result<(), R::Error> {
        todo!()
    }    

    // pub fn read_message_into_slice(&mut self, buf: &mut [u8]) -> Result<usize, ReadDecodedError<R::Error>> {
    //     let mut buf = SliceBuf::new(buf);
    //     let buf = &mut buf;
    //     loop {
    //         let mut b = 0u8;
    //         match self.reader.read(std::slice::from_mut(&mut b)) {
    //             Ok(1) => (),
    //             Ok(_) => unreachable!(),
    //             Err(e) => {
    //                 let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                     Some(DecodeErr::DiscardedBytes(n)) => n,
    //                     Some(_) => unreachable!(),
    //                     None => 0,
    //                 };
    //                 return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
    //             },
    //         }
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(buf.len()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }

    // pub fn read_message_into_vec(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadDecodedError<R::Error>> {
    //     buf.clear();
    //     loop {
    //         let mut b = 0u8;
    //         match self.reader.read(std::slice::from_mut(&mut b)) {
    //             Ok(1) => (),
    //             Ok(_) => unreachable!(),
    //             Err(e) => {
    //                 let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                     Some(DecodeErr::DiscardedBytes(n)) => n,
    //                     Some(_) => unreachable!(),
    //                     None => 0,
    //                 };
    //                 return Err(ReadDecodedError::IoErr(e, num_discarded_bytes))
    //             },
    //         }
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }
}

#[cfg(feature = "embedded-io-async")]
pub struct EIoAsyncReader<R: embedded_io_async::Read> {
    reader: R,
    decoder: DecoderInner,
}


#[cfg(feature = "embedded-io-async")]
impl<R: embedded_io_async::Read> EIoAsyncReader<R> {
    pub async fn read_message(&mut self, buf: &mut impl Buffer) -> Result<usize, ReadDecodedError<R::Error>> {
        todo!()
    }
}


pub struct IterReader<I: Iterator<Item = u8>> {
    iter: I,
    decoder: DecoderInner
}

impl<I: Iterator<Item = u8>> IterReader<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            decoder: Default::default(),
        }
    }

    pub fn read_message(&mut self, buf: &mut impl Buffer) -> Result<usize, ReadDecodedError<Eof>> {
        todo!()
    }

    // pub fn read_message_into_slice(&mut self, buf: &mut [u8]) -> Result<usize, ReadDecodedError<Eof>> {
    //     let mut buf = SliceBuf::new(buf);
    //     let buf = &mut buf;
    //     loop {
    //         let Some(b) = self.iter.next() else {
    //             let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                 Some(DecodeErr::DiscardedBytes(n)) => n,
    //                 Some(_) => unreachable!(),
    //                 None => 0,
    //             };
    //             return Err(ReadDecodedError::IoErr(Eof, num_discarded_bytes))
    //         };
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(buf.len()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }

    // pub fn read_message_into_vec(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadDecodedError<Eof>> {
    //     buf.clear();
    //     loop {
    //         let Some(b) = self.iter.next() else {
    //             let num_discarded_bytes = match self.decoder.finalize(buf) {
    //                 Some(DecodeErr::DiscardedBytes(n)) => n,
    //                 Some(_) => unreachable!(),
    //                 None => 0,
    //             };
    //             return Err(ReadDecodedError::IoErr(Eof, num_discarded_bytes))
    //         };
    //         match self.decoder.push_byte(buf, b) {
    //             Ok(false) => continue,
    //             Ok(true) => return Ok(()),
    //             Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
    //         }
    //     }
    // }
}

// #[test]
// fn test_passing_non_empty_vec() {
//     use hex_literal::hex;
//     let bytes = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
//     let exp = hex!("12345678");
//     let mut reader = std::io::Cursor::new(bytes);
//     let mut reader = IoReader::new(reader);
//     let mut v = vec!(1, 2, 3);
//     let ret = reader.read_message_into_vec(&mut v);
//     assert!(ret.is_ok());
//     assert_eq!(v, exp);
// }