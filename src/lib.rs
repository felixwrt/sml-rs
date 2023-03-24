//! Smart Message Language (SML) parser written in Rust.
//!
//! Modern German power meters periodically send SML-encoded data via an optical interface.
//! The main use-case of this library is to decode that data.
//!
//! See the `transport` module for encoding / decoding the SML transport protocol v1 and the
//! `parser` module for parsing decoded data into SML data structures.
//!
//! Complete examples of how to use the library can be found on github in the `exmples` folder.
//!
//! # Feature flags
//! - **`std`** (default) — Remove this feature to make the library `no_std` compatible.
//! - **`alloc`** (default) — Implementations using allocations (`alloc::Vec` et al.).
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

use core::{convert::Infallible, marker::PhantomData};

use parser::{ParseError};
#[cfg(feature = "alloc")]
use parser::complete::{File, parse};
use transport::DecodeErr;
use util::{Buffer, ArrayBuf};

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;

pub enum ReadDecodedError<E> {
    DecodeErr(DecodeErr),
    IoErr(E),
}

impl<E> From<E> for ReadDecodedError<E> {
    fn from(value: E) -> Self {
        ReadDecodedError::IoErr(value)
    }
}

// impl<E> From<DecodeErr> for ReadDecodedError<E> {
//     fn from(value: DecodeErr) -> Self {
//         ReadDecodedError::DecodeErr(value)
//     }
// }

pub enum ReadParsedError<E> {
    ParseErr(ParseError),
    DecodeErr(DecodeErr),
    IoErr(E),
}

impl<E> From<ReadDecodedError<E>> for ReadParsedError<E> {
    fn from(value: ReadDecodedError<E>) -> Self {
        match value {
            ReadDecodedError::DecodeErr(x) => ReadParsedError::DecodeErr(x),
            ReadDecodedError::IoErr(x) => ReadParsedError::IoErr(x),
        }
    }
}

impl<E> From<ParseError> for ReadParsedError<E> {
    fn from(value: ParseError) -> Self {
        ReadParsedError::ParseErr(value)
    }
}

pub trait ByteSource {
    type Error;

    fn read(&mut self) -> Result<u8, Self::Error>;
}

#[cfg(feature = "std")]
struct IoReader<R: std::io::Read> {
    inner: R
}

#[cfg(feature = "std")]
impl<R> ByteSource for IoReader<R>
where R: std::io::Read {
    type Error = std::io::Error;

    fn read(&mut self) -> Result<u8, Self::Error> {
        let mut buf = [0; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }
}

#[cfg(feature = "embedded_hal")]
struct EhReader<R: embedded_hal::serial::Read<u8, Error = E>, E> {
    inner: R
}

#[cfg(feature = "embedded_hal")]
impl<R, E> ByteSource for EhReader<R, E>
where R: embedded_hal::serial::Read<u8, Error = E> {
    type Error = nb::Error<E>;

    fn read(&mut self) -> Result<u8, Self::Error> {
        self.inner.read()
    }
}

pub struct Eof;

pub struct ArrayReader<const N: usize> {
    inner: [u8; N],
    idx: usize,
}

impl<const N: usize> ByteSource for ArrayReader<N> {
    type Error = Eof;

    fn read(&mut self) -> Result<u8, Self::Error> {
        if self.idx >= N {
            return Err(Eof);
        }
        let b = self.inner[self.idx];
        self.idx += 1;
        Ok(b)
    }
}

pub struct SmlReader<R: ByteSource, Buf: Buffer> {
    reader: R,
    decoder: transport::Decoder<Buf>,
}

#[cfg(feature = "std")]
impl<R: std::io::Read, Buf: Buffer> SmlReader<IoReader<R>, Buf> {
    pub fn from_reader(reader: R) -> Self {
        SmlReader { reader: IoReader { inner: reader }, decoder: transport::Decoder::new() }
    }
}

#[cfg(feature = "embedded_hal")]
impl<R: embedded_hal::serial::Read<u8, Error = E>, E, Buf: Buffer> SmlReader<EhReader<R, E>, Buf> {
        pub fn from_eh_reader(reader: R) -> Self {
        SmlReader { reader: EhReader { inner: reader }, decoder: transport::Decoder::new() }
    }
}

impl<R: ByteSource<Error = E>, E, Buf: Buffer> SmlReader<R, Buf> {
    pub fn read_decoded(&mut self) -> Result<&[u8], ReadDecodedError<E>> {
        loop {
            let b = self.reader.read()?;
            if self.decoder._push_byte(b).map_err(ReadDecodedError::DecodeErr)? {
                return Ok(self.decoder.borrow_buf());
            }
        }
    }

    #[cfg(feature = "alloc")]
    pub fn read_parsed(&mut self) -> Result<File, ReadParsedError<E>> {
        Ok(parse(self.read_decoded()?)?)
    }
}

#[cfg(feature = "alloc")]
type DefaultBuffer = alloc::vec::Vec<u8>;
#[cfg(not(feature = "alloc"))]
type DefaultBuffer = ArrayBuf<{8*1024}>;

struct SmlBuilder<Buf: Buffer=DefaultBuffer> {
    buf: PhantomData<Buf>
}

impl SmlBuilder {
    const fn with_static_buffer<const N: usize>() -> SmlBuilder<ArrayBuf<N>> {
        SmlBuilder { buf: PhantomData }
    }

    #[cfg(feature = "alloc")]
    const fn with_vec_buffer() -> SmlBuilder<alloc::vec::Vec<u8>> {
        SmlBuilder { buf: PhantomData }
    }
}

impl<Buf: Buffer> SmlBuilder<Buf> {
    #[cfg(feature = "std")]
    fn from_reader<R: std::io::Read>(self, reader: R) -> SmlReader<IoReader<R>, Buf> {
        SmlReader { reader: IoReader { inner: reader }, decoder: Default::default() }
    }

    #[cfg(feature = "embedded_hal")]
    fn from_eh_reader<R: embedded_hal::serial::Read<u8, Error = E>, E>(self, reader: R) -> SmlReader<EhReader<R, E>, Buf> {
        SmlReader { reader: EhReader { inner: reader }, decoder: Default::default() }
    }

    fn from_array_reader<const N: usize>(self, reader: [u8; N]) -> SmlReader<ArrayReader<N>, Buf> {
        SmlReader { reader: ArrayReader { inner: reader, idx: 0 }, decoder: Default::default() }
    }
}

#[test]
fn test() {

    // no deps
    let arr = [1,2,3,4,5];
    SmlBuilder::with_static_buffer::<1234>().from_array_reader(arr.clone());
    
    // alloc
    #[cfg(feature = "alloc")]
    SmlBuilder::with_vec_buffer().from_array_reader(arr.clone());

    // eh
    #[cfg(feature = "embedded_hal")]
    struct Pin;
    #[cfg(feature = "embedded_hal")]
    impl embedded_hal::serial::Read<u8> for Pin {
        type Error = i16;

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            Ok(123)
        }
    }
    #[cfg(feature = "embedded_hal")]
    SmlBuilder::with_static_buffer::<1234>().from_eh_reader(Pin);
    
    // eh + alloc
    #[cfg(all(feature = "embedded_hal", feature = "alloc"))]
    SmlBuilder::with_vec_buffer().from_eh_reader(Pin);
    

    // alloc + std
    #[cfg(feature = "std")]
    {
        let v = (0..10).collect::<alloc::vec::Vec<u8>>();
        let reader = std::io::Cursor::new(v);
        let _x = SmlBuilder::with_vec_buffer().from_reader(reader.clone());
        let _x = SmlBuilder::with_static_buffer::<123>().from_reader(reader.clone());
    }
}

#[test]
fn test2() {

}