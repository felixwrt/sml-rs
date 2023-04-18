//! utility stuff

use core::{borrow::Borrow, fmt::Debug, ops::Deref};

pub(crate) static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

pub(crate) mod private {
    pub trait Sealed {}
}

// ===========================================================================
// ===========================================================================
//      `Buffer` trait + impls for `VecBuf` and `ArrayBuf`
// ===========================================================================
// ===========================================================================

/// Interface for byte vectors.
///
/// This train provides is used as an abstraction over different byte vector
/// implementations. It is implemented for static vectors (`ArrayBuf`)
/// and (if the `alloc` feature is used) for dynamic vectors (`alloc::Vec<u8>`).
pub trait Buffer: Default + Deref<Target = [u8]> + private::Sealed {
    /// Appends a byte to the back of the vector.
    ///
    /// Returns `Err` if the vector is full and could not be extended.
    fn push(&mut self, b: u8) -> Result<(), OutOfMemory>;

    /// Shortens the vector, keeping the first len elements and dropping the rest.
    fn truncate(&mut self, len: usize);

    /// Clears the vector, removing all values.
    fn clear(&mut self);

    /// Clones and appends all bytes in a slice to the vector.
    ///
    /// Iterates over the slice `other` and appends each byte to this vector. The `other` vector is traversed in-order.
    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), OutOfMemory>;
}

/// Type alias for `alloc::Vec<u8>`
#[cfg(feature = "alloc")]
pub type VecBuf = alloc::vec::Vec<u8>;

#[cfg(feature = "alloc")]
impl Buffer for VecBuf {
    fn push(&mut self, b: u8) -> Result<(), OutOfMemory> {
        match self.try_reserve(1) {
            Ok(()) => {
                VecBuf::push(self, b);
                Ok(())
            }
            Err(_) => Err(OutOfMemory),
        }
    }

    fn truncate(&mut self, len: usize) {
        VecBuf::truncate(self, len);
    }

    fn clear(&mut self) {
        VecBuf::clear(self);
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), OutOfMemory> {
        match self.try_reserve(other.len()) {
            Ok(()) => {
                VecBuf::extend_from_slice(self, other);
                Ok(())
            }
            Err(_) => Err(OutOfMemory),
        }
    }
}

#[cfg(feature = "alloc")]
impl private::Sealed for VecBuf {}

/// Byte buffer backed by an array.
pub struct ArrayBuf<const N: usize> {
    buffer: [u8; N],
    num_elements: usize,
}

impl<const N: usize> Default for ArrayBuf<N> {
    fn default() -> Self {
        Self {
            buffer: [0; N],
            num_elements: 0,
        }
    }
}

impl<const N: usize> Debug for ArrayBuf<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        (**self).fmt(f)
    }
}

impl<const N: usize> PartialEq for ArrayBuf<N> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<const N: usize> Deref for ArrayBuf<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer[0..self.num_elements]
    }
}

impl<const N: usize> FromIterator<u8> for ArrayBuf<N> {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut buf = ArrayBuf::default();
        for x in iter.into_iter() {
            buf.push(x).unwrap();
        }
        buf
    }
}

impl<const N: usize> Buffer for ArrayBuf<N> {
    fn push(&mut self, b: u8) -> Result<(), OutOfMemory> {
        if self.num_elements == N {
            Err(OutOfMemory)
        } else {
            self.buffer[self.num_elements] = b;
            self.num_elements += 1;
            Ok(())
        }
    }

    fn truncate(&mut self, len: usize) {
        self.num_elements = self.num_elements.min(len);
    }

    fn clear(&mut self) {
        self.num_elements = 0;
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), OutOfMemory> {
        if self.num_elements + other.len() > N {
            return Err(OutOfMemory);
        }
        self.buffer[self.num_elements..][..other.len()].copy_from_slice(other);
        self.num_elements += other.len();
        Ok(())
    }
}

impl<const N: usize> private::Sealed for ArrayBuf<N> {}

/// Error type indicating that an operation failed due to lack of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutOfMemory;

// ===========================================================================
// ===========================================================================
//      `ByteSource` trait + impls
// ===========================================================================
// ===========================================================================

/// Helper trait that allows reading individual bytes
pub trait ByteSource: private::Sealed {
    /// Type of errors that can occur while reading bytes
    type Error: ByteSourceErr;

    /// Tries to read a single byte from the source
    fn read_byte(&mut self) -> Result<u8, Self::Error>;
}

/// Helper trait implemented for Error types of `ByteSource`
pub trait ByteSourceErr: private::Sealed {
    /// Returns whether the error is an end of file (EOF) error
    fn is_eof(&self) -> bool;

    /// Returns whether the error is "would block", which means that reading
    /// can be successfull again later
    fn is_would_block(&self) -> bool;
}

/// Wraps types that implement `std::io::Read` and implements `ByteSource`
#[cfg(feature = "std")]
pub struct IoReader<R>
where
    R: std::io::Read,
{
    inner: R,
}

#[cfg(feature = "std")]
impl<R> IoReader<R>
where
    R: std::io::Read,
{
    pub(crate) fn new(reader: R) -> Self {
        IoReader { inner: reader }
    }
}

#[cfg(feature = "std")]
impl<R> ByteSource for IoReader<R>
where
    R: std::io::Read,
{
    type Error = std::io::Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        let mut b = 0u8;
        self.inner.read_exact(core::slice::from_mut(&mut b))?;
        Ok(b)
    }
}

#[cfg(feature = "std")]
impl<R> private::Sealed for IoReader<R> where R: std::io::Read {}

#[cfg(feature = "std")]
impl ByteSourceErr for std::io::Error {
    fn is_eof(&self) -> bool {
        matches!(self.kind(), std::io::ErrorKind::UnexpectedEof)
    }

    fn is_would_block(&self) -> bool {
        false
    }
}

#[cfg(feature = "std")]
impl private::Sealed for std::io::Error {}

/// Wraps types that implement `embedded_hal::serial::Read<...>` and implements `ByteSource`
#[cfg(feature = "embedded_hal")]
pub struct EhReader<R, E>
where
    R: embedded_hal::serial::Read<u8, Error = E>,
{
    inner: R,
}

#[cfg(feature = "embedded_hal")]
impl<R, E> EhReader<R, E>
where
    R: embedded_hal::serial::Read<u8, Error = E>,
{
    pub(crate) fn new(reader: R) -> Self {
        EhReader { inner: reader }
    }
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

#[cfg(feature = "embedded_hal")]
impl<R, E> private::Sealed for EhReader<R, E> where R: embedded_hal::serial::Read<u8, Error = E> {}

#[cfg(feature = "embedded_hal")]
impl<E> ByteSourceErr for nb::Error<E> {
    fn is_eof(&self) -> bool {
        false
    }

    fn is_would_block(&self) -> bool {
        matches!(self, nb::Error::WouldBlock)
    }
}

#[cfg(feature = "embedded_hal")]
impl<E> private::Sealed for nb::Error<E> {}

/// Error type indicating that the end of the input has been reached
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Eof;

impl ByteSourceErr for Eof {
    fn is_eof(&self) -> bool {
        true
    }

    fn is_would_block(&self) -> bool {
        false
    }
}

impl private::Sealed for Eof {}

/// Wraps byte slices and implements `ByteSource`
pub struct SliceReader<'i> {
    inner: &'i [u8],
    idx: usize,
}

impl<'i> SliceReader<'i> {
    pub(crate) fn new(slice: &'i [u8]) -> Self {
        SliceReader {
            inner: slice,
            idx: 0,
        }
    }
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

impl<'i> private::Sealed for SliceReader<'i> {}

/// Wraps byte iterators and implements `ByteSource`
pub struct IterReader<I, B>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    iter: I,
}

impl<I, B> IterReader<I, B>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    pub(crate) fn new(iter: I) -> Self {
        IterReader { iter }
    }
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

impl<I, B> private::Sealed for IterReader<I, B>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
}

// ===========================================================================
// ===========================================================================
//      Tests
// ===========================================================================
// ===========================================================================

#[cfg(test)]
mod test_arraybuf {
    use crate::util::{Buffer, OutOfMemory};

    use super::ArrayBuf;

    #[test]
    fn test_basic() {
        let mut buf: ArrayBuf<5> = (0..3).collect();
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.push(10), Ok(()));
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.push(20), Ok(()));
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.push(30), Err(OutOfMemory));
        assert_eq!(buf.len(), 5);
        assert_eq!(&*buf, &[0, 1, 2, 10, 20]);
        buf.truncate(1000);
        assert_eq!(&*buf, &[0, 1, 2, 10, 20]);
        buf.truncate(1);
        assert_eq!(&*buf, &[0]);
        buf.truncate(0);
        assert_eq!(&*buf, &[]);
        assert_eq!(buf.extend_from_slice(&[7, 6, 5, 4, 3]), Ok(()));
        assert_eq!(&*buf, &[7, 6, 5, 4, 3]);
        buf.truncate(1);
        assert_eq!(&*buf, &[7]);
        assert_eq!(buf.extend_from_slice(&[10, 11]), Ok(()));
        assert_eq!(&*buf, &[7, 10, 11]);
        assert_eq!(buf.extend_from_slice(&[25, 26, 27]), Err(OutOfMemory));
        buf.clear();
        assert_eq!(&*buf, &[]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_debug() {
        use alloc::format;
        let mut buf: ArrayBuf<5> = (10..13).collect();
        assert_eq!(&format!("{:?}", buf), "[10, 11, 12]");
        assert_eq!(&format!("{:x?}", buf), "[a, b, c]");
        buf.clear();
        assert_eq!(&format!("{:?}", buf), "[]");
    }

    #[test]
    fn test_from_iter() {
        let buf: ArrayBuf<5> = (0..3).collect();
        assert_eq!(&*buf, &[0, 1, 2]);
        let buf: ArrayBuf<3> = (0..3).collect();
        assert_eq!(&*buf, &[0, 1, 2]);
    }

    #[test]
    #[should_panic]
    fn test_from_panic() {
        let _: ArrayBuf<2> = (0..3).collect();
    }

    #[test]
    fn test_eq() {
        let mut buf_a: ArrayBuf<5> = (0..3).collect();
        let mut buf_b: ArrayBuf<5> = (0..3).collect();
        assert_eq!(buf_a, buf_b);
        buf_b.push(123).unwrap();
        assert_ne!(buf_a, buf_b);
        buf_b.push(199).unwrap();
        assert_ne!(buf_a, buf_b);
        buf_a.truncate(3);
        buf_b.truncate(3);
        assert_eq!(buf_a, buf_b);
        buf_a.clear();
        assert_ne!(buf_a, buf_b);
        buf_b.clear();
        assert_eq!(buf_a, buf_b);
    }

    #[test]
    fn test_n0() {
        let mut buf = ArrayBuf::<0>::default();
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.push(30), Err(OutOfMemory));
    }
}
