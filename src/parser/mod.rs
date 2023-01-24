//! Parsers for SML messages.
//!
//! This module contains parsers for the SML protocol. The `complete` module contains an easy to use allocating
//! parser. The `streaming` module contains a flexible non-allocating parser. See the discussion below for a
//! comparison of the two parsers:
//!
//! # Which parser should I choose?
//!
//! The SML protocol defines two data structures that can hold multiple elements. An SML File can
//! contain multiple SML Messages and the "SML_GetList.Res" message contains a list of values.
//! Because of these two elements, the size of an SML File cannot be known at compile time.
//!
//! The parser in the `complete` module uses dynamic memory allocations `alloc::vec::Vec` for SML
//! Messages and SML Values. This makes the usage straight-forward and if you're using `sml-rs` on
//! a hosted platform, this is most likely the parser you'll want to use.
//!
//! The parser in the `streaming` module works differently and therefore doesn't require dynamic
//! memory allocations. Instead of returning a single data structure representing the whole SML File,
//! this parser produces a stream of events that each have a size known at compile time. Depending on the
//! input, the parser will produce a different number of events, which is how different numbers of SML
//! Messages / Values can be handled. If you're using `sml-rs` on a microcontroller and don't want to use
//! an allocator, this is the parser you'll want to use.
//!
//! # Examples
//!
#![cfg_attr(
    feature = "alloc",
    doc = r##"
## Using `complete::parse`

```rust
# use sml_rs::parser::complete;
let bytes: &[u8] = &[ /*...*/ ];

println!("{:#?}", complete::parse(&bytes).expect("error while parsing"));
```

Output (stripped-down to the relevant parts):
```text
File {
    messages: [
        Message {
            message_body: OpenResponse {
                ref_time: SecIndex(23876784),
                ...
            },
            ...
        },
        Message {
            message_body: GetListResponse {
                val_list: [
                    ListEntry { ... },
                    ListEntry { ... },
                    ListEntry { ... },
                ],
            },
            ...
        },
        Message {
            message_body: CloseResponse,
            ...
        },
    ],
}
```
"##
)]
//!
//! ## Using `streaming::Parser`
//! ```rust
//! # use sml_rs::parser::streaming;
//! let bytes: &[u8] = &[ /*...*/ ];
//!
//! let parser = streaming::Parser::new(bytes);
//! for item in parser {
//!     println!("- {:#?}", item.expect("error while parsing"));
//! }
//! ```
//!
//! Output (stripped-down to the relevant parts):
//! ```text
//! - MessageStart(MessageStart {
//!     message_body: OpenResponse {
//!         ref_time: SecIndex(23876784),
//!         ...
//!     },
//!     ...
//! })
//! - MessageStart(MessageStart {
//!     message_body: GetListResponseStart {
//!         num_values: 3,
//!         ...
//!     },
//!     ...
//! })
//! - ListEntry(ListEntry { ... })
//! - ListEntry(ListEntry { ... })
//! - ListEntry(ListEntry { ... })
//! - GetListResponseEnd(GetListResponseEnd)
//! - MessageStart(MessageStart {
//!     message_body: CloseResponse,
//!     ...
//! })
//! ```
//!
//!

use core::{fmt::Debug, ops::Deref};

use tlf::TypeLengthField;

pub mod common;
#[cfg(feature = "alloc")]
pub mod complete;
mod num;
mod octet_string;
pub mod streaming;
mod tlf;

pub use tlf::TlfParseError;

pub use octet_string::OctetStr;

/// Error type used by the parser
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// There are additional bytes in the input while the parser expects EOF
    LeftoverInput,
    /// The parser expected additional bytes but encountered an EOF
    UnexpectedEOF,
    /// An error occurred while parsing a `TypeLengthField`
    InvalidTlf(TlfParseError),
    /// TLF mismatch while parsing struct / enum
    TlfMismatch(&'static str),
    /// CRC mismatch,
    CrcMismatch,
    /// Expected to find 0x00 as message end marker, got something else
    MsgEndMismatch,
    /// Got a variant id that isn't known. This means it's either invalid or not supported (yet) by the parser
    UnexpectedVariant,
}

type ResTy<'i, O> = Result<(&'i [u8], O), ParseError>;
#[allow(dead_code)]
type ResTyComplete<'i, O> = Result<O, ParseError>;

/// SmlParse is the main trait used to parse bytes into SML data structures.
pub(crate) trait SmlParse<'i>
where
    Self: Sized,
{
    /// Tries to parse an instance of `Self` from a byte slice.
    ///
    /// On success, returns the remaining input and the parsed instance of `Self`.
    fn parse(input: &'i [u8]) -> ResTy<Self>;

    /// Tries to parse an instance of `Self` from a byte slice and returns an error if there are leftover bytes.
    ///
    /// On success, returns the parsed instance of `Self`.
    fn parse_complete(input: &'i [u8]) -> ResTyComplete<Self> {
        let (input, x) = Self::parse(input)?;
        if !input.is_empty() {
            return Err(ParseError::LeftoverInput);
        }
        Ok(x)
    }
}

pub(crate) trait SmlParseTlf<'i>
where
    Self: Sized,
{
    fn check_tlf(tlf: &TypeLengthField) -> bool;

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self>;
}

impl<'i, T: SmlParseTlf<'i>> SmlParse<'i> for T {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (input, tlf) = TypeLengthField::parse(input)?;
        if !Self::check_tlf(&tlf) {
            return Err(ParseError::TlfMismatch(core::any::type_name::<Self>()));
        }
        Self::parse_with_tlf(input, &tlf)
    }
}

impl<'i, T: SmlParse<'i>> SmlParse<'i> for Option<T> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        if let Some(0x01u8) = input.first() {
            Ok((&input[1..], None))
        } else {
            let (input, x) = T::parse(input)?;
            Ok((input, Some(x)))
        }
    }
}

fn take_byte(input: &[u8]) -> ResTy<u8> {
    if input.is_empty() {
        return Err(ParseError::UnexpectedEOF);
    }
    Ok((&input[1..], input[0]))
}

fn take<const N: usize>(input: &[u8]) -> ResTy<&[u8; N]> {
    if input.len() < N {
        return Err(ParseError::UnexpectedEOF);
    }
    Ok((&input[N..], input[..N].try_into().unwrap()))
}

fn take_n(input: &[u8], n: usize) -> ResTy<&[u8]> {
    if input.len() < n {
        return Err(ParseError::UnexpectedEOF);
    }
    Ok((&input[n..], &input[..n]))
}

fn map<O1, O2>(val: ResTy<O1>, mut f: impl FnMut(O1) -> O2) -> ResTy<O2> {
    val.map(|(input, x)| (input, f(x)))
}

struct OctetStrFormatter<'i>(&'i [u8]);

// formats a slice using the compact single-line output even when the parent element should be formatted using "{:#?}"
impl<'i> Debug for OctetStrFormatter<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

struct NumberFormatter<T: Debug, U: Deref<Target = T>>(U);

impl<T: Debug, U: Deref<Target = T>> Debug for NumberFormatter<T, U> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}{}", self.0.deref(), core::any::type_name::<T>())
    }
}
