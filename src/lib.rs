//! This crate is a WIP implementation of the Smart Message Language (SML).
//!
//! Properties:
//! - `no_std` by default, optional support for allocations using the `alloc` feature flag.
//!
//! # Feature flags
//! - **`alloc`** — Implementations using allocations (`alloc::Vec` et al.).
//!
#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

use core::ops::Deref;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod transport;
pub mod parser;

static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

/// Type alias for `heapless::Vec<u8>`
pub type ArrayBuf<const N: usize> = heapless::Vec<u8, N>;
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

impl<const N: usize> Buffer for ArrayBuf<N> {
    fn push(&mut self, b: u8) -> Result<(), OutOfMemory> {
        ArrayBuf::push(self, b).map_err(|_| OutOfMemory)
    }

    fn truncate(&mut self, len: usize) {
        ArrayBuf::truncate(self, len);
    }

    fn clear(&mut self) {
        ArrayBuf::clear(self);
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), OutOfMemory> {
        ArrayBuf::extend_from_slice(self, other).map_err(|_| OutOfMemory)
    }
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

/// Error type indicating that an operation failed due to lack of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutOfMemory;
