//! This module implements the SML parser

pub mod num;
pub mod octet_string;
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

// fn take<const N: usize>(input: &[u8]) -> ResTy<&[u8; N]> {
//     if input.len() < N {
//         return Err(ParseError::UnexpectedEOF);
//     }
//     Ok((&input[N..], input[..N].try_into().unwrap()))
// }

fn take_n(input: &[u8], n: usize) -> ResTy<&[u8]> {
    if input.len() < n {
        return Err(ParseError::UnexpectedEOF);
    }
    Ok((&input[n..], &input[..n]))
}
