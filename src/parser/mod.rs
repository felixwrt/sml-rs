//! This module implements the SML parser

use core::{fmt::Debug, ops::Deref};

use self::tlf::TypeLengthField;

pub mod domain;
pub mod num;
pub mod octet_string;
pub mod streaming;
pub mod tlf;

/// Error type used by the parser
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// There are additional bytes in the input while the parser expects EOF
    LeftoverInput,
    /// The parser expected additional bytes but encountered an EOF
    UnexpectedEOF,
    /// Type field of TLF doesn't match OctetString
    OctetStrTlfTypeMismatch,
    /// The length field of a TLF overflowed
    TlfLengthOverflow,
    /// The TLF uses values reserved for future usage
    TlfReserved,
    /// The length field of a TLF underflowed
    TlfLengthUnderflow,
    /// The type field of a byte following the first TLF byte isn't set to `000`
    TlfNextByteTypeMismatch,
    /// The TLF's type field contains an invalid value
    TlfInvalidTy,
    /// TLF doesn't match the number type being parsed
    NumTlfMismatch,
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
type ResTyComplete<'i, O> = Result<O, ParseError>;

/// SmlParse is the main trait used to parse bytes into SML data structures.
pub trait SmlParse<'i>
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
