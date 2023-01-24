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

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;
