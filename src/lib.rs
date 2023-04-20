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
//! - **`nb`** - Enables non-blocking APIs using the `nb` crate.
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(unsafe_code)]
#![warn(missing_docs)]

use core::{borrow::Borrow, marker::PhantomData};

// #[cfg(feature = "alloc")]
use parser::complete::{parse, File};
use parser::streaming::Parser;
use parser::ParseError;
use transport::{DecodeErr, DecoderReader, ReadDecodedError};
use util::{ArrayBuf, Buffer};

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;

use util::ByteSource;

/// Error returned by functions parsing sml data read from a reader
#[derive(Debug)]
pub enum ReadParsedError<E>
where
    E: core::fmt::Debug,
{
    /// Error while parsing
    ParseErr(ParseError),
    /// Error while decoding the data (e.g. checksum mismatch)
    DecodeErr(DecodeErr),
    /// Error while reading from the internal byte source
    ///
    /// (inner_error, num_discarded_bytes)
    IoErr(E, usize),
}

impl<E> From<ReadDecodedError<E>> for ReadParsedError<E>
where
    E: core::fmt::Debug,
{
    fn from(value: ReadDecodedError<E>) -> Self {
        match value {
            ReadDecodedError::DecodeErr(x) => ReadParsedError::DecodeErr(x),
            ReadDecodedError::IoErr(x, num_discarded) => ReadParsedError::IoErr(x, num_discarded),
        }
    }
}

impl<E> From<ParseError> for ReadParsedError<E>
where
    E: core::fmt::Debug,
{
    fn from(value: ParseError) -> Self {
        ReadParsedError::ParseErr(value)
    }
}

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
/// ```
/// # #[cfg(feature = "std")] {
/// # use sml_rs::{parser::complete::File, SmlReader};
/// use std::fs;
/// let f = fs::File::open("sample.bin").unwrap();
/// let mut reader = SmlReader::from_reader(f);
/// match reader.read::<File>() {
///     Ok(x) => println!("Got result: {:#?}", x),
///     Err(e) => println!("Error: {:?}", e),
/// }
/// # }
/// ```
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
/// # #[cfg(feature = "alloc")] {
/// # use sml_rs::SmlReader;
/// let data = [1, 2, 3, 4, 5];
/// let reader_2 = SmlReader::with_vec_buffer().from_iterator(&data);
/// # }
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
    decoder: DecoderReader<Buf, R>,
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
            decoder: DecoderReader::new(util::IoReader::new(reader)),
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
            decoder: DecoderReader::new(util::EhReader::new(reader)),
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
            decoder: DecoderReader::new(util::SliceReader::new(reader)),
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
            decoder: DecoderReader::new(util::IterReader::new(iter.into_iter())),
        }
    }
}

impl<R, E, Buf> SmlReader<R, Buf>
where
    R: ByteSource<Error = E>,
    E: core::fmt::Debug,
    Buf: Buffer,
{
    /// Reads, decodes and possibly parses sml data.
    ///
    /// ```
    /// # use sml_rs::{SmlReader, DecodedBytes};
    /// let data = include_bytes!("../sample.bin");
    /// let mut reader = SmlReader::from_slice(data.as_slice());
    ///
    /// let bytes = reader.read::<DecodedBytes>();
    /// assert!(matches!(bytes, Ok(bytes)));
    /// let bytes = reader.read::<DecodedBytes>();
    /// assert!(matches!(bytes, Err(_)));
    /// ```
    ///
    /// This method can be used to parse sml data into several representations.
    /// See the module documentation for more information.
    ///
    /// When reading from a finite data source (such as a file containing a certain
    /// number of transmissions), it's easier to use [`next`](SmlReader::next) instead,
    /// which returns `None` when an EOF is read when trying to read the next transmission.
    ///
    /// See also [`read_nb`](DecoderReader::read_nb), which provides a convenient API for
    /// non-blocking byte sources.
    pub fn read<'i, T>(&'i mut self) -> Result<T, T::Error>
    where
        T: SmlParse<'i, E>,
    {
        T::parse_from(self.decoder.read())
    }

    /// Tries to read, decode and possibly parse sml data.
    ///
    /// ```
    /// # use sml_rs::{SmlReader, DecodedBytes};
    /// let data = include_bytes!("../sample.bin");
    /// let mut reader = SmlReader::from_slice(data.as_slice());
    ///
    /// let bytes = reader.next::<DecodedBytes>();
    /// assert!(matches!(bytes, Some(Ok(bytes))));
    /// let bytes = reader.next::<DecodedBytes>();
    /// assert!(matches!(bytes, None));
    /// ```
    ///
    /// This method can be used to parse sml data into several representations.
    /// See the module documentation for more information.
    ///
    /// When reading from a data source that will provide data infinitely (such
    /// as from a serial port), it's easier to use [`read`](SmlReader::read) instead.
    ///
    /// See also [`next_nb`](SmlReader::next_nb), which provides a convenient API for
    /// non-blocking byte sources.
    pub fn next<'i, T>(&'i mut self) -> Option<Result<T, T::Error>>
    where
        T: SmlParse<'i, E>,
    {
        Some(T::parse_from(self.decoder.next()?))
    }

    /// Reads, decodes and possibly parses sml data (non-blocking).
    ///
    /// ```
    /// # use sml_rs::{SmlReader, DecodedBytes};
    /// let data = include_bytes!("../sample.bin");
    /// let mut reader = SmlReader::from_slice(data.as_slice());
    ///
    /// let bytes = nb::block!(reader.read_nb::<DecodedBytes>());
    /// assert!(matches!(bytes, Ok(bytes)));
    /// let bytes = nb::block!(reader.read_nb::<DecodedBytes>());
    /// assert!(matches!(bytes, Err(_)));
    /// ```
    ///
    /// Same as [`read`](SmlReader::read) except that it returns `nb::Result`.
    /// If reading from the byte source indicates that data isn't available yet,
    /// this method returns `Err(nb::Error::WouldBlock)`.
    ///
    /// Using `nb::Result` allows this method to be awaited using the `nb::block!` macro.
    ///
    /// *This function is available only if sml-rs is built with the `"nb"` or `"embedded_hal"` features.*
    #[cfg(feature = "nb")]
    pub fn read_nb<'i, T>(&'i mut self) -> nb::Result<T, T::Error>
    where
        T: SmlParse<'i, E>,
    {
        // TODO: this could probably be written better
        let res = match self.decoder.read_nb() {
            Ok(x) => Ok(x),
            Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
            Err(nb::Error::Other(e)) => Err(e),
        };
        T::parse_from(res).map_err(nb::Error::Other)
    }

    /// Tries to read, decode and possibly parse sml data (non-blocking).
    ///
    /// ```
    /// # use sml_rs::{SmlReader, DecodedBytes};
    /// let data = include_bytes!("../sample.bin");
    /// let mut reader = SmlReader::from_slice(data.as_slice());
    ///
    /// let bytes = nb::block!(reader.next_nb::<DecodedBytes>());
    /// assert!(matches!(bytes, Ok(Some(bytes))));
    /// let bytes = nb::block!(reader.next_nb::<DecodedBytes>());
    /// assert!(matches!(bytes, Ok(None)));
    /// ```
    ///
    /// Same as [`next`](SmlReader::next) except that it returns `nb::Result`.
    /// If reading from the byte source indicates that data isn't available yet,
    /// this method returns `Err(nb::Error::WouldBlock)`.
    ///
    /// Using `nb::Result` allows this method to be awaited using the `nb::block!` macro.
    ///
    /// *This function is available only if sml-rs is built with the `"nb"` or `"embedded_hal"` features.*
    #[cfg(feature = "nb")]
    pub fn next_nb<'i, T>(&'i mut self) -> nb::Result<Option<T>, T::Error>
    where
        T: SmlParse<'i, E>,
    {
        // TODO: this could probably be written better
        let res = match self.decoder.next_nb() {
            Ok(None) => return Ok(None),
            Ok(Some(x)) => Ok(x),
            Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
            Err(nb::Error::Other(e)) => Err(e),
        };
        T::parse_from(res).map(Some).map_err(nb::Error::Other)
    }
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
            decoder: DecoderReader::new(util::IoReader::new(reader)),
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
            decoder: DecoderReader::new(util::EhReader::new(reader)),
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
            decoder: DecoderReader::new(util::SliceReader::new(reader)),
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
            decoder: DecoderReader::new(util::IterReader::new(iter.into_iter())),
        }
    }
}

/// Helper trait implemented for types that built from decoded bytes.
///
/// *This trait is not meant to be used directly, use [`SmlReader::read`] and
/// [`SmlReader::next`] instead.*
pub trait SmlParse<'i, IoErr>: Sized + util::private::Sealed {
    /// The error produced if parsing fails or the input contained an error.
    type Error;

    /// Takes the result of decoding and parses it into the resulting type.
    ///
    /// *This function is not meant to be used directly, use [`SmlReader::read`] and
    /// [`SmlReader::next`] instead.*
    fn parse_from(value: Result<&'i [u8], ReadDecodedError<IoErr>>) -> Result<Self, Self::Error>;
}

/// Type alias for decoded bytes.
pub type DecodedBytes<'i> = &'i [u8];

impl<'i, E> SmlParse<'i, E> for DecodedBytes<'i>
where
    E: core::fmt::Debug,
{
    type Error = ReadDecodedError<E>;

    fn parse_from(value: Result<&'i [u8], ReadDecodedError<E>>) -> Result<Self, Self::Error> {
        value
    }
}

impl<'i> util::private::Sealed for DecodedBytes<'i> {}

#[cfg(feature = "alloc")]
impl<'i, E> SmlParse<'i, E> for File<'i>
where
    E: core::fmt::Debug,
{
    type Error = ReadParsedError<E>;

    fn parse_from(value: Result<&'i [u8], ReadDecodedError<E>>) -> Result<Self, Self::Error> {
        Ok(parse(value?)?)
    }
}

impl<'i> util::private::Sealed for File<'i> {}

impl<'i, E> SmlParse<'i, E> for Parser<'i>
where
    E: core::fmt::Debug,
{
    type Error = ReadDecodedError<E>;

    fn parse_from(value: Result<&'i [u8], ReadDecodedError<E>>) -> Result<Self, Self::Error> {
        Ok(Parser::new(value?))
    }
}

impl<'i> util::private::Sealed for Parser<'i> {}

#[test]
fn test_smlreader_construction() {
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
fn test_smlreader_eh_construction() {
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

mod read_tests {
    #[test]
    fn test_smlreader_reading() {
        // check that different types can be used with read and next
        use super::{DecodedBytes, File, Parser, SmlReader};

        let bytes = [1, 2, 3, 4];
        let mut reader = SmlReader::from_slice(&bytes);

        let _ = reader.read::<DecodedBytes>();
        let _: Result<DecodedBytes, _> = reader.read();

        let _ = reader.read::<File>();
        let _ = reader.read::<Parser>();

        let _ = reader.next::<DecodedBytes>();
        let _ = reader.next::<File>();
        let _ = reader.next::<Parser>();
    }

    #[test]
    #[cfg(feature = "nb")]
    fn test_smlreader_reading_nb() {
        // check that different types can be used with read_nb and next_nb
        use super::{DecodedBytes, File, Parser, SmlReader};

        let bytes = [1, 2, 3, 4];
        let mut reader = SmlReader::from_slice(&bytes);

        let _ = reader.next_nb::<DecodedBytes>();
        let _ = reader.next_nb::<File>();
        let _ = reader.next_nb::<Parser>();

        let _ = reader.read_nb::<DecodedBytes>();
        let _ = reader.read_nb::<File>();
        let _ = reader.read_nb::<Parser>();
    }
}
