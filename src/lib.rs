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
//! - **`embedded_hal`** — Allows using pins implementing `embedded_hal::serial::Read` in [`SmlReader`](SmlReader::from_eh_reader).
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

use core::{borrow::Borrow, marker::PhantomData};

// #[cfg(feature = "alloc")]
// use parser::complete::{parse, File};
// use parser::ParseError;
use transport::DecodeErr;
use util::{ArrayBuf, Buffer};

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;

use util::ByteSource;

// --------- ERROR TYPES

#[derive(Debug, PartialEq)]
/// Error used when decoding sml data read from a reader
pub enum ReadDecodedError<E>
where
    E: core::fmt::Debug,
{
    /// Error while decoding (e.g. checksum mismatch)
    DecodeErr(DecodeErr),
    /// Error while reading from the underlying reader
    IoErr(E),
}

impl<E> From<E> for ReadDecodedError<E>
where
    E: core::fmt::Debug,
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

// #[derive(Debug)]
// pub enum ReadParsedError<E>
// where
//     E: core::fmt::Debug
// {
//     ParseErr(ParseError),
//     DecodeErr(DecodeErr),
//     IoErr(E),
// }

// impl<E> From<ReadDecodedError<E>> for ReadParsedError<E>
// where
//     E: core::fmt::Debug
// {
//     fn from(value: ReadDecodedError<E>) -> Self {
//         match value {
//             ReadDecodedError::DecodeErr(x) => ReadParsedError::DecodeErr(x),
//             ReadDecodedError::IoErr(x) => ReadParsedError::IoErr(x),
//         }
//     }
// }

// impl<E> From<ParseError> for ReadParsedError<E>
// where
//     E: core::fmt::Debug
// {
//     fn from(value: ParseError) -> Self {
//         ReadParsedError::ParseErr(value)
//     }
// }

// ===========================================================================
// ===========================================================================
//      `SmlReader` + impls
// ===========================================================================
// ===========================================================================

/// Main API of `sml-rs`
///
/// `SmlReader` is used to read sml data. It allows reading from various data
/// sources and can produce different output depending on the use-case.
///
/// ## Example
///
/// The following example shows how to parse an sml data set from a file:
///
/// // ```
/// // # use sml_rs::SmlReader;
/// // use std::fs::File;
/// // let f = File::open("sample.bin").unwrap();
/// // let mut reader = SmlReader::from_reader(f);
/// // match reader.read_parsed() {
/// //     Ok(x) => println!("Got result: {:#?}", x),
/// //     Err(e) => println!("Error: {:?}", e),
/// // }
/// // ```
/// ### Data Source
///
/// The `SmlReader` struct can be used with several kinds of data providers:
///
/// | Constructor (`SmlReader::...`)          | Expected data type | Usage examples |
/// |-----------------------------------------------------|-----------|------------|
/// |[`from_reader`](SmlReader::from_reader) **¹**             | `impl std::io::Read` | files, sockets, serial ports (see `serialport-rs` crate) |
/// |[`from_eh_reader`](SmlReader::from_eh_reader) **²** | `impl embedded_hal::serial::Read<u8>` | microcontroller pins |
/// |[`from_slice`](SmlReader::from_slice)                | `&[u8]` | arrays, vectors, ... |
/// |[`from_iterator`](SmlReader::from_iterator)                  | `impl IntoIterator<Item = impl Borrow<u8>>)` | anything that can be turned into an iterator over bytes |
///
/// ***¹** requires feature `std` (on by default); **²** requires optional feature `embedded_hal`*
///
/// ### Internal Buffer
///
/// `SmlReader` reads sml messages into an internal buffer. By default, a static
/// buffer with a size of 8 KiB is used, which should be more than enough for
/// typical messages.
///
/// It is possible to use a different static buffer size or use a dynamically
/// allocated buffer that can grow as necessary. `SmlReader` provides two associated
/// functions for this purpose:
///
/// - [`SmlReader::with_static_buffer<N>()`](SmlReader::with_static_buffer)
/// - [`SmlReader::with_vec_buffer()`](SmlReader::with_vec_buffer) *(requires feature `alloc` (on by default))*
///
/// These functions return a builder object ([`SmlReaderBuilder`](SmlReaderBuilder)) that provides methods to create an [`SmlReader`](SmlReader)
/// from the different data sources shown above.
///
/// **Examples**
///
/// Creating a reader with a static 1KiB buffer from a slice:
///
/// ```
/// # use sml_rs::SmlReader;
/// let data = [1, 2, 3, 4, 5];
/// let reader = SmlReader::with_static_buffer::<1024>().from_slice(&data);
/// ```
///
/// Creating a reader with a dynamically-sized buffer from an iterable:
///
/// ```
/// # use sml_rs::SmlReader;
/// let data = [1, 2, 3, 4, 5];
/// let reader_2 = SmlReader::with_vec_buffer().from_iterator(&data);
/// ```
///
/// ### Target Type
///
/// **TODO!**
///
pub struct SmlReader<R, Buf>
where
    R: ByteSource,
    Buf: Buffer,
{
    reader: R,
    decoder: transport::Decoder<Buf>,
}

pub(crate) type DummySmlReader = SmlReader<util::SliceReader<'static>, ArrayBuf<0>>;

impl DummySmlReader {
    /// Returns a builder with a static internal buffer of size `N`.
    ///
    /// Use the `from_*` methods on the builder to create an `SmlReader`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data = [1, 2, 3];
    /// let reader = SmlReader::with_static_buffer::<1024>().from_slice(&data);
    /// ```
    pub fn with_static_buffer<const N: usize>() -> SmlReaderBuilder<ArrayBuf<N>> {
        SmlReaderBuilder { buf: PhantomData }
    }

    /// Returns a builder with a dynamically-sized internal buffer.
    ///
    /// Use the `from_*` methods on the builder to create an `SmlReader`.
    ///
    /// *This function is available only if sml-rs is built with the `"alloc"` feature.*
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data = [1, 2, 3];
    /// let reader = SmlReader::with_vec_buffer().from_slice(&data);
    /// ```
    #[cfg(feature = "alloc")]
    pub fn with_vec_buffer() -> SmlReaderBuilder<alloc::vec::Vec<u8>> {
        SmlReaderBuilder { buf: PhantomData }
    }

    /// Build an `SmlReader` from a type implementing `std::io::Read`.
    ///
    /// *This function is available only if sml-rs is built with the `"std"` feature.*
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data = [1, 2, 3];
    /// let cursor = std::io::Cursor::new(data);  // implements std::io::Read
    /// let reader = SmlReader::from_reader(cursor);
    /// ```
    #[cfg(feature = "std")]
    pub fn from_reader<R>(reader: R) -> SmlReader<util::IoReader<R>, DefaultBuffer>
    where
        R: std::io::Read,
    {
        SmlReader {
            reader: util::IoReader::new(reader),
            decoder: transport::Decoder::new(),
        }
    }

    /// Build an `SmlReader` from a type implementing `embedded_hal::serial::Read<u8>`.
    ///
    /// *This function is available only if sml-rs is built with the `"embedded-hal"` feature.*
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// // usually provided by hardware abstraction layers (HALs) for specific chips
    /// // let pin = ...;
    /// # struct Pin;
    /// # impl embedded_hal::serial::Read<u8> for Pin {
    /// #     type Error = ();
    /// #     fn read(&mut self) -> nb::Result<u8, Self::Error> { Ok(123) }
    /// # }
    /// # let pin = Pin;
    ///
    /// let reader = SmlReader::from_eh_reader(pin);
    /// ```
    #[cfg(feature = "embedded_hal")]
    pub fn from_eh_reader<R, E>(reader: R) -> SmlReader<util::EhReader<R, E>, DefaultBuffer>
    where
        R: embedded_hal::serial::Read<u8, Error = E>,
    {
        SmlReader {
            reader: util::EhReader::new(reader),
            decoder: transport::Decoder::new(),
        }
    }

    /// Build an `SmlReader` from a slice of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data: &[u8] = &[1, 2, 3];
    /// let reader = SmlReader::from_slice(data);
    /// ```
    pub fn from_slice(reader: &[u8]) -> SmlReader<util::SliceReader<'_>, DefaultBuffer> {
        SmlReader {
            reader: util::SliceReader::new(reader),
            decoder: Default::default(),
        }
    }

    /// Build an `SmlReader` from a type that can be turned into a byte iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data: [u8; 3] = [1, 2, 3];
    /// let reader = SmlReader::from_iterator(data.clone());      // [u8; 3]
    /// let reader = SmlReader::from_iterator(&data);             // &[u8; 3]
    /// let reader = SmlReader::from_iterator(data.as_slice());   // &[u8]
    /// let reader = SmlReader::from_iterator(data.iter());       // impl Iterator<Item = &u8>
    /// let reader = SmlReader::from_iterator(data.into_iter());  // impl Iterator<Item = u8>
    /// ```
    pub fn from_iterator<B, I>(
        iter: I,
    ) -> SmlReader<util::IterReader<I::IntoIter, B>, DefaultBuffer>
    where
        I: IntoIterator<Item = B>,
        B: Borrow<u8>,
    {
        SmlReader {
            reader: util::IterReader::new(iter.into_iter()),
            decoder: transport::Decoder::new(),
        }
    }
}

impl<R, E, Buf> SmlReader<R, Buf>
where
    R: ByteSource<Error = E>,
    E: core::fmt::Debug,
    Buf: Buffer,
{
    /// Reads an sml transmission (encoded with the sml transport v1) from the internal reader
    ///
    /// Decodes the transport v1 and returns the contained data as a slice of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::{SmlReader, util::Eof, ReadDecodedError};
    /// let bytes = [0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x12, 0x34, 0x56, 0x78, 0x1b, 0x1b, 0x1b, 0x1b, 0x1a, 0x00, 0xb8, 0x7b];
    /// let mut reader = SmlReader::from_slice(&bytes);
    /// assert_eq!(reader.read_decoded_bytes(), Ok([0x12, 0x34, 0x56, 0x78].as_slice()));
    /// assert_eq!(reader.read_decoded_bytes(), Err(ReadDecodedError::IoErr(Eof)))
    /// ```
    pub fn read_decoded_bytes(&mut self) -> Result<&[u8], ReadDecodedError<E>> {
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

    // TODO: implement this!
    // #[cfg(feature = "alloc")]
    // pub fn read_parsed(&mut self) -> Result<File, ReadParsedError<E>> {
    //     Ok(parse(self.read_decoded_bytes()?)?)
    // }
}

type DefaultBuffer = ArrayBuf<{ 8 * 1024 }>;

/// Builder struct for `SmlReader` that allows configuring the internal buffer type.
///
/// See [here](SmlReader#internal-buffer) for an explanation of the different internal
/// buffer types and how to use the builder to customize them.
pub struct SmlReaderBuilder<Buf: Buffer> {
    buf: PhantomData<Buf>,
}

impl<Buf: Buffer> Clone for SmlReaderBuilder<Buf> {
    fn clone(&self) -> Self {
        Self { buf: PhantomData }
    }
}

impl<Buf: Buffer> SmlReaderBuilder<Buf> {
    /// Build an `SmlReader` from a type implementing `std::io::Read`.
    ///
    /// *This function is available only if sml-rs is built with the `"std"` feature.*
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data = [1, 2, 3];
    /// let cursor = std::io::Cursor::new(data);  // implements std::io::Read
    /// let reader = SmlReader::with_static_buffer::<1024>().from_reader(cursor);
    /// ```
    #[cfg(feature = "std")]
    pub fn from_reader<R: std::io::Read>(self, reader: R) -> SmlReader<util::IoReader<R>, Buf> {
        SmlReader {
            reader: util::IoReader::new(reader),
            decoder: Default::default(),
        }
    }

    /// Build an `SmlReader` from a type implementing `embedded_hal::serial::Read<u8>`.
    ///
    /// *This function is available only if sml-rs is built with the `"embedded-hal"` feature.*
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// // usually provided by hardware abstraction layers (HALs) for specific chips
    /// // let pin = ...;
    /// # struct Pin;
    /// # impl embedded_hal::serial::Read<u8> for Pin {
    /// #     type Error = ();
    /// #     fn read(&mut self) -> nb::Result<u8, Self::Error> { Ok(123) }
    /// # }
    /// # let pin = Pin;
    ///
    /// let reader = SmlReader::with_static_buffer::<1024>().from_eh_reader(pin);
    /// ```
    #[cfg(feature = "embedded_hal")]
    pub fn from_eh_reader<R: embedded_hal::serial::Read<u8, Error = E>, E>(
        self,
        reader: R,
    ) -> SmlReader<util::EhReader<R, E>, Buf> {
        SmlReader {
            reader: util::EhReader::new(reader),
            decoder: Default::default(),
        }
    }

    /// Build an `SmlReader` from a slice of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data: &[u8] = &[1, 2, 3];
    /// let reader = SmlReader::with_static_buffer::<1024>().from_slice(data);
    /// ```
    pub fn from_slice(self, reader: &[u8]) -> SmlReader<util::SliceReader<'_>, Buf> {
        SmlReader {
            reader: util::SliceReader::new(reader),
            decoder: Default::default(),
        }
    }

    /// Build an `SmlReader` from a type that can be turned into a byte iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sml_rs::SmlReader;
    /// let data: [u8; 3] = [1, 2, 3];
    /// let builder = SmlReader::with_static_buffer::<1024>();
    /// let reader = builder.clone().from_iterator(data.clone());      // [u8; 3]
    /// let reader = builder.clone().from_iterator(&data);             // &[u8; 3]
    /// let reader = builder.clone().from_iterator(data.as_slice());   // &[u8]
    /// let reader = builder.clone().from_iterator(data.iter());       // impl Iterator<Item = &u8>
    /// let reader = builder.clone().from_iterator(data.into_iter());  // impl Iterator<Item = u8>
    /// ```
    pub fn from_iterator<B, I>(self, iter: I) -> SmlReader<util::IterReader<I::IntoIter, B>, Buf>
    where
        I: IntoIterator<Item = B>,
        B: Borrow<u8>,
    {
        SmlReader {
            reader: util::IterReader::new(iter.into_iter()),
            decoder: Default::default(),
        }
    }
}

#[test]
fn test_smlreader() {
    let arr = [1, 2, 3, 4, 5];

    // no deps

    // using default buffer
    SmlReader::from_slice(&arr);
    SmlReader::from_iterator(&arr);
    SmlReader::from_iterator(arr.iter().map(|x| x + 1));
    #[cfg(feature = "std")]
    SmlReader::from_reader(std::io::Cursor::new(&arr));

    // using static buffer
    SmlReader::with_static_buffer::<1234>().from_slice(&arr);
    SmlReader::with_static_buffer::<1234>().from_iterator(arr.iter().map(|x| x + 1));
    #[cfg(feature = "std")]
    SmlReader::with_static_buffer::<1234>().from_reader(std::io::Cursor::new(&arr));

    // using dynamic buffer
    #[cfg(feature = "alloc")]
    SmlReader::with_vec_buffer().from_slice(&arr);
    #[cfg(feature = "alloc")]
    SmlReader::with_vec_buffer().from_iterator(arr.iter().map(|x| x + 1));
    #[cfg(feature = "std")]
    SmlReader::with_vec_buffer().from_reader(std::io::Cursor::new(&arr));
}

#[test]
#[cfg(feature = "embedded_hal")]
fn test_smlreader_eh() {
    // dummy struct implementing `Read`
    struct Pin;
    impl embedded_hal::serial::Read<u8> for Pin {
        type Error = i16;

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            Ok(123)
        }
    }

    // using default buffer
    SmlReader::from_eh_reader(Pin);

    // using static buffer
    SmlReader::with_static_buffer::<1234>().from_eh_reader(Pin);

    // using dynamic buffer
    #[cfg(feature = "alloc")]
    SmlReader::with_vec_buffer().from_eh_reader(Pin);
}
