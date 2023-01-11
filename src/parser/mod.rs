//! This module implements the SML parser
//! 
//! # Examples
//! 
//! ```
//! use sml_rs::parser::{parse, File, Message, MessageBody, CloseResponse};
//! 
//! let bytes = [0x76, 0x5, 0xdd, 0x43, 0x44, 0x0, 0x62, 0x0, 0x62, 0x0, 0x72, 0x63, 0x2, 0x1, 0x71, 0x1, 0x63, 0xfd, 0x56, 0x0];
//! 
//! // parse the input data
//! let result = parse(&bytes);
//! 
//! let expected = File {
//!     messages: vec![
//!         Message { 
//!             transaction_id: &[221, 67, 68, 0], 
//!             group_no: 0, 
//!             abort_on_error: 0, 
//!             message_body: MessageBody::CloseResponse(CloseResponse { 
//!                 global_signature: None 
//!             })
//!         }
//!     ]
//! }
//! assert_eq!(result, Ok(expected))
//! ```

use core::{fmt::Debug, ops::Deref};

use tlf::TypeLengthField;

mod domain;
mod num;
mod octet_string;
mod tlf;

pub use tlf::TlfParseError;

pub use octet_string::OctetStr;

#[cfg(feature = "alloc")]
pub use octet_string::OctetString;

pub use domain::*;

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

/// Parses a slice of bytes into an SML message.
///
/// *This function is available only if sml-rs is built with the `"alloc"` feature.*
#[cfg(feature = "alloc")]
pub fn parse(input: &[u8]) -> Result<File, ParseError> {
    File::parse_complete(input)
}

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
