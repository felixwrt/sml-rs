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

use core::{marker::PhantomData, slice, borrow::Borrow};

#[cfg(feature = "alloc")]
use parser::complete::{parse, File};
use parser::ParseError;
use transport::DecodeErr;
use util::{ArrayBuf, Buffer};

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;

// --------- ERROR TYPES

#[derive(Debug)]
pub enum ReadDecodedError<E> 
where 
    E: core::fmt::Debug
{
    DecodeErr(DecodeErr),
    IoErr(E),
}

impl<E> From<E> for ReadDecodedError<E> 
where 
    E: core::fmt::Debug
{
    fn from(value: E) -> Self {
        ReadDecodedError::IoErr(value)
    }
}

// impl<E> From<DecodeErr> for ReadDecodedError<E> {
//     fn from(value: DecodeErr) -> Self {
//         ReadDecodedError::DecodeErr(value)
//     }
// }

#[derive(Debug)]
pub enum ReadParsedError<E>
where 
    E: core::fmt::Debug
{
    ParseErr(ParseError),
    DecodeErr(DecodeErr),
    IoErr(E),
}

impl<E> From<ReadDecodedError<E>> for ReadParsedError<E> 
where 
    E: core::fmt::Debug
{
    fn from(value: ReadDecodedError<E>) -> Self {
        match value {
            ReadDecodedError::DecodeErr(x) => ReadParsedError::DecodeErr(x),
            ReadDecodedError::IoErr(x) => ReadParsedError::IoErr(x),
        }
    }
}

impl<E> From<ParseError> for ReadParsedError<E> 
where 
    E: core::fmt::Debug
{
    fn from(value: ParseError) -> Self {
        ReadParsedError::ParseErr(value)
    }
}

// --------- BYTE SOURCE


/// Helper trait that allows reading individual bytes
pub trait ByteSource {
    /// Type of errors that can occur while reading bytes
    type Error;

    /// Tries to read a single byte from the source
    fn read_byte(&mut self) -> Result<u8, Self::Error>;
}

/// Wraps types that implement `std::io::Read` and implements `ByteSource`
#[cfg(feature = "std")]
pub struct IoReader<R> 
where 
    R: std::io::Read
{
    inner: R,
}

#[cfg(feature = "std")]
impl<R> ByteSource for IoReader<R>
where
    R: std::io::Read,
{
    type Error = std::io::Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        let mut b = 0u8;
        self.inner.read_exact(slice::from_mut(&mut b))?;
        Ok(b)
    }
}

/// Wraps types that implement `embedded_hal::serial::Read<...>` and implements `ByteSource`
#[cfg(feature = "embedded_hal")]
pub struct EhReader<R, E> 
where
    R: embedded_hal::serial::Read<u8, Error = E>
{
    inner: R,
}

#[cfg(feature = "embedded_hal")]
impl<R, E> ByteSource for EhReader<R, E>
where
    R: embedded_hal::serial::Read<u8, Error = E>,
{
    type Error = nb::Error<E>;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        self.inner.read()
    }
}

/// Error type indicating that the end of the input has been reached
pub struct Eof;

/// Wraps byte slices and implements `ByteSource`
pub struct SliceReader<'i> {
    inner: &'i [u8],
    idx: usize,
}

impl<'i> ByteSource for SliceReader<'i> {
    type Error = Eof;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        if self.idx >= self.inner.len() {
            return Err(Eof);
        }
        let b = self.inner[self.idx];
        self.idx += 1;
        Ok(b)
    }
}

/// Wraps byte iterators and implements `ByteSource`
pub struct IterReader<I, B>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    iter: I,
}

impl<I, B> ByteSource for IterReader<I, B>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    type Error = Eof;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        match self.iter.next() {
            Some(x) => Ok(*x.borrow()),
            None => Err(Eof),
        }
    }
}

// ------------- SmlReader

pub struct SmlReader<R, Buf> 
where
    R: ByteSource,
    Buf: Buffer,
{
    reader: R,
    decoder: transport::Decoder<Buf>,
}

impl<R, Buf> SmlReader<R, Buf> 
where 
    R: ByteSource,
    Buf: Buffer{
    pub fn new(reader: R, buffer: Buf) -> Self {
        SmlReader { 
            reader, 
            decoder: transport::Decoder::from_buf(buffer) 
        }
    }
}

/// Foo bar baz
pub type DummySmlReader = SmlReader<SliceReader<'static>, ArrayBuf<0>>;

impl DummySmlReader {
    pub const fn with_static_buffer<const N: usize>() -> SmlReaderBuilder<ArrayBuf<N>> {
        SmlReaderBuilder { buf: PhantomData }
    }

    #[cfg(feature = "alloc")]
    pub const fn with_vec_buffer() -> SmlReaderBuilder<alloc::vec::Vec<u8>> {
        SmlReaderBuilder { buf: PhantomData }
    }

    #[cfg(feature = "std")]
    pub fn from_reader<R>(reader: R) -> SmlReader<IoReader<R>, DefaultBuffer> 
    where 
        R: std::io::Read
    {
        SmlReader { 
            reader: IoReader { inner: reader }, 
            decoder: transport::Decoder::new() 
        }
    }

    #[cfg(feature = "embedded_hal")]
    pub fn from_eh_reader<R, E>(reader: R) -> SmlReader<EhReader<R, E>, DefaultBuffer>
    where
        R: embedded_hal::serial::Read<u8, Error = E>
    {
        SmlReader {
            reader: EhReader { inner: reader },
            decoder: transport::Decoder::new(),
        }
    }

    pub fn from_iter<B, I>(iter: I) -> SmlReader<IterReader<I::IntoIter, B>, DefaultBuffer>
    where
        I: IntoIterator<Item = B>,
        B: Borrow<u8>,
    {
        SmlReader {
            reader: IterReader { iter: iter.into_iter() },
            decoder: transport::Decoder::new(),
        }
    }
}

impl<R, E, Buf> SmlReader<R, Buf> 
where
    R: ByteSource<Error = E>, 
    E: core::fmt::Debug, 
    Buf: Buffer
{
    pub fn read_decoded(&mut self) -> Result<&[u8], ReadDecodedError<E>> {
        loop {
            let b = self.reader.read_byte()?;
            if self
                .decoder
                ._push_byte(b)
                .map_err(ReadDecodedError::DecodeErr)?
            {
                return Ok(self.decoder.borrow_buf());
            }
        }
    }

    #[cfg(feature = "alloc")]
    pub fn read_parsed(&mut self) -> Result<File, ReadParsedError<E>> {
        Ok(parse(self.read_decoded()?)?)
    }
}

type DefaultBuffer = ArrayBuf<{ 8 * 1024 }>;

pub struct SmlReaderBuilder<Buf: Buffer = DefaultBuffer> {
    buf: PhantomData<Buf>,
}

impl SmlReaderBuilder {
    pub const fn with_static_buffer<const N: usize>() -> SmlReaderBuilder<ArrayBuf<N>> {
        SmlReaderBuilder { buf: PhantomData }
    }

    #[cfg(feature = "alloc")]
    pub const fn with_vec_buffer() -> SmlReaderBuilder<alloc::vec::Vec<u8>> {
        SmlReaderBuilder { buf: PhantomData }
    }
}

impl<Buf: Buffer> SmlReaderBuilder<Buf> {
    #[cfg(feature = "std")]
    pub fn from_reader<R: std::io::Read>(self, reader: R) -> SmlReader<IoReader<R>, Buf> {
        SmlReader {
            reader: IoReader { inner: reader },
            decoder: Default::default(),
        }
    }

    #[cfg(feature = "embedded_hal")]
    pub fn from_eh_reader<R: embedded_hal::serial::Read<u8, Error = E>, E>(
        self,
        reader: R,
    ) -> SmlReader<EhReader<R, E>, Buf> {
        SmlReader {
            reader: EhReader { inner: reader },
            decoder: Default::default(),
        }
    }

    pub fn from_slice<'i>(
        self,
        reader: &'i [u8],
    ) -> SmlReader<SliceReader<'i>, Buf> {
        SmlReader {
            reader: SliceReader {
                inner: reader,
                idx: 0,
            },
            decoder: Default::default(),
        }
    }

    pub fn from_iter<B, I>(iter: I) -> SmlReader<IterReader<I::IntoIter, B>, Buf>
    where
        I: IntoIterator<Item = B>,
        B: Borrow<u8>,
    {
        SmlReader {
            reader: IterReader { iter: iter.into_iter() },
            decoder: Default::default(),
        }
    }
}

#[test]
fn test() {
    // no deps
    let arr = [1, 2, 3, 4, 5];
    SmlReader::with_static_buffer::<1234>().from_slice(&arr);
    SmlReader {
        reader: SliceReader {
            inner: &arr,
            idx: 0,
        },
        decoder: crate::transport::Decoder::<ArrayBuf<1234>>::new(),
    };

    let reader = SmlReader::from_reader(std::io::Cursor::new(arr));

    let reader = SmlReader::from_iter(arr);

    // alloc
    #[cfg(feature = "alloc")]
    SmlReader::with_vec_buffer().from_slice(&arr);

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
    SmlReader::with_static_buffer::<1234>().from_eh_reader(Pin);

    // eh + alloc
    #[cfg(all(feature = "embedded_hal", feature = "alloc"))]
    SmlReader::with_vec_buffer().from_eh_reader(Pin);

    // alloc + std
    #[cfg(feature = "std")]
    {
        let v = (0..10).collect::<alloc::vec::Vec<u8>>();
        let reader = std::io::Cursor::new(v);
        let _x = SmlReader::with_vec_buffer().from_reader(reader.clone());
        let _x = SmlReader::with_static_buffer::<123>().from_reader(reader.clone());
    }
}

#[test]
fn test2() {}
