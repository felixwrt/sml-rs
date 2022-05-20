#![no_std]

use core::ops::Deref;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod transport;

static CRC_X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

pub type ArrayBuf<const N: usize> = heapless::Vec<u8, N>;
#[cfg(feature = "alloc")]
pub type VecBuf = alloc::vec::Vec<u8>;

pub trait Buffer: Default + Deref<Target = [u8]> {
    fn push(&mut self, b: u8) -> Result<(), u8>;

    fn truncate(&mut self, len: usize);

    fn clear(&mut self);

    #[allow(clippy::result_unit_err)]
    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), ()>;
}

impl<const N: usize> Buffer for ArrayBuf<N> {
    fn push(&mut self, b: u8) -> Result<(), u8> {
        ArrayBuf::push(self, b)
    }

    fn truncate(&mut self, len: usize) {
        ArrayBuf::truncate(self, len)
    }

    fn clear(&mut self) {
        ArrayBuf::clear(self)
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), ()> {
        ArrayBuf::extend_from_slice(self, other)
    }
}

#[cfg(feature = "alloc")]
impl Buffer for VecBuf {
    fn push(&mut self, b: u8) -> Result<(), u8> {
        match self.try_reserve(1) {
            Ok(()) => {
                VecBuf::push(self, b);
                Ok(())
            }
            Err(_) => Err(b),
        }
    }

    fn truncate(&mut self, len: usize) {
        VecBuf::truncate(self, len)
    }

    fn clear(&mut self) {
        VecBuf::clear(self)
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), ()> {
        match self.try_reserve(other.len()) {
            Ok(()) => {
                VecBuf::extend_from_slice(self, other);
                Ok(())
            }
            Err(_) => Err(()),
        }
    }
}
