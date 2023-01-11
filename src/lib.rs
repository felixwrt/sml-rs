//! This crate is a WIP implementation of the Smart Message Language (SML).
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

#[cfg(feature = "alloc")]
pub mod parser;
pub mod transport;
pub mod util;
