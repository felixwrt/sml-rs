//! module containing the `DecodeReader` and related implementation

use core::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{DecodeErr, Decoder};
use crate::util::{Buffer, ByteSource, ByteSourceErr, ErrKind};

/// Error type used by the `DecoderReader`
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq)]
pub enum ReadDecodedError<IoErr> {
    /// Error while decoding the data (e.g. checksum mismatch)
    DecodeErr(DecodeErr),
    /// Error while reading from the internal byte source
    ///
    /// (inner_error, num_discarded_bytes)
    IoErr(IoErr, usize),
}

impl fmt::Display for ReadDecodedError<fmt::Error> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ReadDecodedError<fmt::Error> {}

/// Decode transmissions read from a byte source
pub struct DecoderReader<B, R>
where
    B: Buffer,
    R: ByteSource,
{
    decoder: Decoder<B>,
    reader: R,
}

impl<B, R> DecoderReader<B, R>
where
    B: Buffer,
    R: ByteSource,
{
    /// Create a new decoder wrapping the provided reader.
    pub fn new(reader: R) -> Self {
        DecoderReader {
            decoder: Default::default(),
            reader,
        }
    }

    /// Reads and decodes a transmission
    ///
    /// On success, returns the decoded transmission (`Ok(bytes)`). Otherwise, returns errors
    /// (which can either be decoding errors or errors returned from the byte source).
    ///
    /// When reading from a finite data source (such as a file containing a certain
    /// number of transmissions), it's easier to use [`next`](DecoderReader::next) instead,
    /// which handles EOFs correctly.
    ///
    /// See also [`read_nb`](DecoderReader::read_nb), which provides a convenient API for
    /// non-blocking byte sources.
    pub fn read(&mut self) -> Result<&[u8], ReadDecodedError<R::ReadError>> {
        loop {
            match self.reader.read_byte() {
                Ok(b) => match self.decoder._push_byte(b) {
                    Ok(false) => continue,
                    Ok(true) => return Ok(self.decoder.borrow_buf()),
                    Err(e) => return Err(ReadDecodedError::DecodeErr(e)),
                },
                Err(e) => {
                    let discarded_bytes = match e.kind() {
                        ErrKind::Eof | ErrKind::Other => {
                            // reset the decoder and return how many bytes were discarded
                            self.decoder.reset()
                        }
                        ErrKind::WouldBlock => 0,
                    };
                    // return the error
                    return Err(ReadDecodedError::IoErr(e, discarded_bytes));
                }
            }
        }
    }

    /// Tries to read and decode a transmission
    ///
    /// On success, returns the decoded transmission (`Some(Ok(bytes))`). Returns
    /// `None` if the reader returns EOF immediately. Otherwise, returns errors
    /// (which can either be decoding errors or errors returned from the byte source).
    ///
    /// When reading from a data source that will provide data infinitely (such
    /// as from a serial port), it's easier to use [`read`](DecoderReader::read) instead.
    ///
    /// See also [`next_nb`](DecoderReader::next_nb), which provides a convenient API for
    /// non-blocking byte sources.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<&[u8], ReadDecodedError<R::ReadError>>> {
        match self.read() {
            Err(ReadDecodedError::IoErr(e, 0)) if e.is_eof() => None,
            x => Some(x),
        }
    }

    /// Reads and decodes a transmission (non-blocking)
    ///
    /// Same as [`read`](DecoderReader::read) except that it returns `nb::Result`.
    /// If reading from the byte source indicates that data isn't available yet,
    /// this method returns `Err(nb::Error::WouldBlock)`.
    ///
    /// Using `nb::Result` allows this method to be awaited using the `nb::block!` macro.
    ///
    /// *This function is available only if sml-rs is built with the `"nb"` or `"embedded-hal-02"` features.*
    #[cfg(feature = "nb")]
    pub fn read_nb(&mut self) -> nb::Result<&[u8], ReadDecodedError<R::ReadError>> {
        self.read().map_err(|e| match e {
            ReadDecodedError::IoErr(io_err, _) if io_err.is_would_block() => nb::Error::WouldBlock,
            other => nb::Error::Other(other),
        })
    }

    /// Tries to read and decode a transmission (non-blocking)
    ///
    /// Same as [`next`](DecoderReader::next) except that it returns `nb::Result`.
    /// If reading from the byte source indicates that data isn't available yet,
    /// this method returns `Err(nb::Error::WouldBlock)`.
    ///
    /// Using `nb::Result` allows this method to be awaited using the `nb::block!` macro.
    ///
    /// *This function is available only if sml-rs is built with the `"nb"` or `"embedded-hal-02"` features.*
    #[cfg(feature = "nb")]
    pub fn next_nb(&mut self) -> nb::Result<Option<&[u8]>, ReadDecodedError<R::ReadError>> {
        match self.read_nb() {
            Err(nb::Error::Other(ReadDecodedError::IoErr(e, 0))) if e.is_eof() => Ok(None),
            Err(e) => Err(e),
            Ok(x) => Ok(Some(x)),
        }
    }
}

#[cfg(test)]
mod decoder_reader_tests {
    use core::iter::once;

    use crate::util::ArrayBuf;

    use super::*;
    use hex_literal::hex;

    struct TestReader<I>
    where
        I: Iterator<Item = Result<u8, TestReaderErr>>,
    {
        iter: I,
    }

    impl<I> ByteSource for TestReader<I>
    where
        I: Iterator<Item = Result<u8, TestReaderErr>>,
    {
        type ReadError = TestReaderErr;

        fn read_byte(&mut self) -> Result<u8, Self::ReadError> {
            self.iter.next().unwrap_or(Err(TestReaderErr::Eof))
        }
    }

    impl<I> crate::util::private::Sealed for TestReader<I> where
        I: Iterator<Item = Result<u8, TestReaderErr>>
    {
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum TestReaderErr {
        Eof,
        Other,
        WouldBlock,
    }

    impl ByteSourceErr for TestReaderErr {
        fn kind(&self) -> ErrKind {
            match self {
                TestReaderErr::Eof => ErrKind::Eof,
                TestReaderErr::Other => ErrKind::Other,
                TestReaderErr::WouldBlock => ErrKind::WouldBlock,
            }
        }
    }

    impl crate::util::private::Sealed for TestReaderErr {}

    fn decoder_from<I>(iter: I) -> DecoderReader<ArrayBuf<1024>, TestReader<I>>
    where
        I: Iterator<Item = Result<u8, TestReaderErr>>,
    {
        DecoderReader {
            decoder: Default::default(),
            reader: TestReader { iter },
        }
    }

    #[test]
    fn successful_read_then_eof() {
        let data = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b");
        let mut dr = decoder_from(data.into_iter().map(Ok));
        assert_eq!(dr.next(), Some(Ok(hex!("12345678").as_slice())));
        assert_eq!(dr.next(), None);
    }

    #[test]
    fn eof_while_parsing() {
        let data = hex!("1b1b1b1b 01010101 12");
        let mut dr = decoder_from(data.into_iter().map(Ok));
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::Eof, 9)))
        );
        assert_eq!(dr.next(), None);
        assert_eq!(dr.next(), None);
    }

    #[test]
    fn err_while_parsing_ok_afterwards() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("1b1b1b1b 01010101 12345678 1b1b1b1b 1a00b87b")
            .into_iter()
            .map(Ok);
        let all_data = data.chain(once(Err(TestReaderErr::Other))).chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::Other, 9)))
        );
        assert_eq!(dr.next(), Some(Ok(hex!("12345678").as_slice())));
        assert_eq!(dr.next(), None);
        assert_eq!(dr.next(), None);
    }

    #[test]
    fn would_block_while_parsing() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = data
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::WouldBlock, 0)))
        );
        assert_eq!(dr.next(), Some(Ok(hex!("12345678").as_slice())));
        assert_eq!(dr.next(), None);
        assert_eq!(dr.next(), None);
    }

    #[test]
    fn would_block_before_parsing() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = once(Err(TestReaderErr::WouldBlock))
            .chain(data)
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::WouldBlock, 0)))
        );
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::WouldBlock, 0)))
        );
        assert_eq!(dr.next(), Some(Ok(hex!("12345678").as_slice())));
        assert_eq!(dr.next(), None);
        assert_eq!(dr.next(), None);
    }

    #[test]
    fn immediate_err() {
        let all_data = once(Err(TestReaderErr::Other));
        let mut dr = decoder_from(all_data);
        assert_eq!(
            dr.next(),
            Some(Err(ReadDecodedError::IoErr(TestReaderErr::Other, 0)))
        );
        assert_eq!(dr.next(), None);
        assert_eq!(dr.next(), None);
    }

    #[test]
    #[cfg(feature = "nb")]
    fn read_nb() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = once(Err(TestReaderErr::WouldBlock))
            .chain(data)
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(dr.read_nb(), Err(nb::Error::WouldBlock));
        assert_eq!(dr.read_nb(), Err(nb::Error::WouldBlock));
        assert_eq!(dr.read_nb(), Ok(hex!("12345678").as_slice()));
        assert_eq!(
            dr.read_nb(),
            Err(nb::Error::Other(ReadDecodedError::IoErr(
                TestReaderErr::Eof,
                0
            )))
        );
        assert_eq!(
            dr.read_nb(),
            Err(nb::Error::Other(ReadDecodedError::IoErr(
                TestReaderErr::Eof,
                0
            )))
        );
    }

    #[test]
    #[cfg(feature = "nb")]
    fn next_nb() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = once(Err(TestReaderErr::WouldBlock))
            .chain(data)
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(dr.next_nb(), Err(nb::Error::WouldBlock));
        assert_eq!(dr.next_nb(), Err(nb::Error::WouldBlock));
        assert_eq!(dr.next_nb(), Ok(Some(hex!("12345678").as_slice())));
        assert_eq!(dr.next_nb(), Ok(None));
        assert_eq!(dr.next_nb(), Ok(None));
    }

    #[test]
    #[cfg(feature = "nb")]
    fn nb_block_next_nb() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = once(Err(TestReaderErr::WouldBlock))
            .chain(data)
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(
            nb::block!(dr.next_nb()),
            Ok(Some(hex!("12345678").as_slice()))
        );
        assert_eq!(nb::block!(dr.next_nb()), Ok(None));
        assert_eq!(nb::block!(dr.next_nb()), Ok(None));
    }

    #[test]
    #[cfg(feature = "nb")]
    fn nb_block_read_nb() {
        let data = hex!("1b1b1b1b 01010101 12").into_iter().map(Ok);
        let data2 = hex!("345678 1b1b1b1b 1a00b87b").into_iter().map(Ok);
        let all_data = once(Err(TestReaderErr::WouldBlock))
            .chain(data)
            .chain(once(Err(TestReaderErr::WouldBlock)))
            .chain(data2);
        let mut dr = decoder_from(all_data);
        assert_eq!(nb::block!(dr.read_nb()), Ok(hex!("12345678").as_slice()));
        assert_eq!(
            nb::block!(dr.read_nb()),
            Err(ReadDecodedError::IoErr(TestReaderErr::Eof, 0))
        );
        assert_eq!(
            nb::block!(dr.read_nb()),
            Err(ReadDecodedError::IoErr(TestReaderErr::Eof, 0))
        );
    }
}
