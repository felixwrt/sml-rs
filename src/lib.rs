//! This crate is a WIP implementation of the Smart Message Language (SML).
//!
//! Properties:
//! - `no_std` by default, optional support for allocations using the `alloc` feature flag.
//!
//! # Feature flags
//! - **`alloc`** — Implementations using allocations (`alloc::Vec` et al.).
//!
// #![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod parser;
pub mod transport;
pub mod util;

