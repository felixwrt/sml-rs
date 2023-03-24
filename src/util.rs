//! utility stuff

use core::{ops::Deref, fmt::Debug};

pub(crate) static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

/// Type alias for `alloc::Vec<u8>`
#[cfg(feature = "alloc")]
pub type VecBuf = alloc::vec::Vec<u8>;

/// Interface for byte vectors.
///
/// This train provides is used as an abstraction over different byte vector
/// implementations. It is implemented for static vectors (`heapless::Vec<u8>`)
/// and (if the `alloc` feature is used( for dynamic vectors (`alloc::Vec<u8>`).
pub trait Buffer: Default + Deref<Target = [u8]> {
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


/// Byte buffer backed by an array.
pub struct ArrayBuf<const N: usize> {
    buffer: [u8; N],
    num_elements: usize,
}

impl<const N: usize> Default for ArrayBuf<N> {
    fn default() -> Self {
        Self { 
            buffer: [0; N], 
            num_elements: 0 
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

impl<const N: usize>  Buffer for ArrayBuf<N> {
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

/// Error type indicating that an operation failed due to lack of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutOfMemory;
